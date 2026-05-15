use std::collections::BTreeSet;
use std::fmt;

pub const PCS_QUERY_PLAN_SEGMENT_ID: u32 = 10_001;

const PCS_QUERY_PLAN_MAGIC: [u8; 4] = *b"pqs0";
const PCS_QUERY_PLAN_VERSION: u32 = 1;
const HEADER_BYTES: usize = 4 + 4 + 4;
const UNIT_HEADER_BYTES: usize = 4 + 4;
const QUERY_BYTES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsQueryPlanSegment {
    pub units: Vec<PcsQueryPlanUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsQueryPlanUnit {
    pub unit_index: u32,
    pub queries: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsQueryPlanSegmentError {
    InvalidMagic,
    UnsupportedVersion { version: u32 },
    UnexpectedEof { needed: usize, available: usize },
    TrailingBytes { trailing: usize },
    EmptyUnits,
    EmptyQueries { unit_index: u32 },
    DuplicateUnitIndex { unit_index: u32 },
    LengthOverflow,
}

impl fmt::Display for PcsQueryPlanSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid PCS query plan segment magic"),
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported PCS query plan segment version: {version}")
            }
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "truncated PCS query plan segment: needed {needed}, available {available}"
            ),
            Self::TrailingBytes { trailing } => {
                write!(f, "trailing PCS query plan segment bytes: {trailing}")
            }
            Self::EmptyUnits => write!(f, "PCS query plan segment has no units"),
            Self::EmptyQueries { unit_index } => {
                write!(f, "PCS query plan unit {unit_index} has no queries")
            }
            Self::DuplicateUnitIndex { unit_index } => {
                write!(f, "duplicate PCS query plan unit index: {unit_index}")
            }
            Self::LengthOverflow => write!(f, "PCS query plan segment length overflow"),
        }
    }
}

impl std::error::Error for PcsQueryPlanSegmentError {}

pub fn encode_pcs_query_plan_segment(
    value: &PcsQueryPlanSegment,
) -> Result<Vec<u8>, PcsQueryPlanSegmentError> {
    validate_pcs_query_plan_segment(value)?;
    let unit_count =
        u32::try_from(value.units.len()).map_err(|_| PcsQueryPlanSegmentError::LengthOverflow)?;
    let expected_len = value.units.iter().try_fold(HEADER_BYTES, |acc, unit| {
        unit.queries
            .len()
            .checked_mul(QUERY_BYTES)
            .and_then(|bytes| bytes.checked_add(UNIT_HEADER_BYTES))
            .and_then(|bytes| bytes.checked_add(acc))
            .ok_or(PcsQueryPlanSegmentError::LengthOverflow)
    })?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&PCS_QUERY_PLAN_MAGIC);
    write_u32(&mut out, PCS_QUERY_PLAN_VERSION);
    write_u32(&mut out, unit_count);
    for unit in &value.units {
        let query_count = u32::try_from(unit.queries.len())
            .map_err(|_| PcsQueryPlanSegmentError::LengthOverflow)?;
        write_u32(&mut out, unit.unit_index);
        write_u32(&mut out, query_count);
        for query in &unit.queries {
            write_u64(&mut out, *query);
        }
    }
    Ok(out)
}

pub fn parse_pcs_query_plan_segment(
    bytes: &[u8],
) -> Result<PcsQueryPlanSegment, PcsQueryPlanSegmentError> {
    let mut reader = SegmentReader::new(bytes);
    let magic = reader.read_array::<4>()?;
    if magic != PCS_QUERY_PLAN_MAGIC {
        return Err(PcsQueryPlanSegmentError::InvalidMagic);
    }
    let version = reader.read_u32()?;
    if version != PCS_QUERY_PLAN_VERSION {
        return Err(PcsQueryPlanSegmentError::UnsupportedVersion { version });
    }
    let unit_count = reader.read_u32()? as usize;
    if unit_count == 0 {
        return Err(PcsQueryPlanSegmentError::EmptyUnits);
    }

    let mut units = Vec::with_capacity(unit_count);
    for _ in 0..unit_count {
        let unit_index = reader.read_u32()?;
        let query_count = reader.read_u32()? as usize;
        let mut queries = Vec::with_capacity(query_count);
        for _ in 0..query_count {
            queries.push(reader.read_u64()?);
        }
        units.push(PcsQueryPlanUnit {
            unit_index,
            queries,
        });
    }
    reader.finish()?;

    let out = PcsQueryPlanSegment { units };
    validate_pcs_query_plan_segment(&out)?;
    Ok(out)
}

fn validate_pcs_query_plan_segment(
    value: &PcsQueryPlanSegment,
) -> Result<(), PcsQueryPlanSegmentError> {
    if value.units.is_empty() {
        return Err(PcsQueryPlanSegmentError::EmptyUnits);
    }
    let mut seen = BTreeSet::new();
    for unit in &value.units {
        if unit.queries.is_empty() {
            return Err(PcsQueryPlanSegmentError::EmptyQueries {
                unit_index: unit.unit_index,
            });
        }
        if !seen.insert(unit.unit_index) {
            return Err(PcsQueryPlanSegmentError::DuplicateUnitIndex {
                unit_index: unit.unit_index,
            });
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

    fn read_u32(&mut self) -> Result<u32, PcsQueryPlanSegmentError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, PcsQueryPlanSegmentError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], PcsQueryPlanSegmentError> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(PcsQueryPlanSegmentError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(PcsQueryPlanSegmentError::UnexpectedEof {
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

    fn finish(&self) -> Result<(), PcsQueryPlanSegmentError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(PcsQueryPlanSegmentError::TrailingBytes {
                trailing: self.bytes.len() - self.offset,
            })
        }
    }
}

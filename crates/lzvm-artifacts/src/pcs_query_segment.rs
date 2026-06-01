use std::collections::BTreeSet;
use std::fmt;

pub const PCS_QUERY_PLAN_SEGMENT_ID: u32 = 10_001;

const PCS_QUERY_PLAN_MAGIC: [u8; 4] = *b"pqs0";
const PCS_QUERY_PLAN_V1_VERSION: u32 = 1;
const PCS_QUERY_PLAN_V2_VERSION: u32 = 2;
const HEADER_BYTES: usize = 4 + 4 + 4;
const V1_UNIT_HEADER_BYTES: usize = 4 + 4;
const V2_UNIT_HEADER_BYTES: usize = 4 + 4 + 4;
const QUERY_BYTES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsQueryPlanSegment {
    pub units: Vec<PcsQueryPlanUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsQueryPlanUnit {
    pub unit_index: u32,
    pub trace_instance_index: u32,
    pub queries: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsQueryPlanSegmentError {
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
    DuplicateUnitIndex {
        unit_index: u32,
    },
    DuplicateUnitIdentity {
        unit_index: u32,
        trace_instance_index: u32,
    },
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
            Self::DuplicateUnitIdentity {
                unit_index,
                trace_instance_index,
            } => write!(
                f,
                "duplicate PCS query plan unit identity: unit {unit_index}, trace instance {trace_instance_index}"
            ),
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
    let version = query_plan_version(value);
    let unit_header_bytes = unit_header_bytes(version)?;
    let expected_len = value.units.iter().try_fold(HEADER_BYTES, |acc, unit| {
        unit.queries
            .len()
            .checked_mul(QUERY_BYTES)
            .and_then(|bytes| bytes.checked_add(unit_header_bytes))
            .and_then(|bytes| bytes.checked_add(acc))
            .ok_or(PcsQueryPlanSegmentError::LengthOverflow)
    })?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&PCS_QUERY_PLAN_MAGIC);
    write_u32(&mut out, version);
    write_u32(&mut out, unit_count);
    for unit in &value.units {
        let query_count = u32::try_from(unit.queries.len())
            .map_err(|_| PcsQueryPlanSegmentError::LengthOverflow)?;
        write_u32(&mut out, unit.unit_index);
        if version == PCS_QUERY_PLAN_V2_VERSION {
            write_u32(&mut out, unit.trace_instance_index);
        }
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
    if !matches!(
        version,
        PCS_QUERY_PLAN_V1_VERSION | PCS_QUERY_PLAN_V2_VERSION
    ) {
        return Err(PcsQueryPlanSegmentError::UnsupportedVersion { version });
    }
    let unit_count = usize::try_from(reader.read_u32()?)
        .map_err(|_| PcsQueryPlanSegmentError::LengthOverflow)?;
    if unit_count == 0 {
        return Err(PcsQueryPlanSegmentError::EmptyUnits);
    }
    let unit_header_bytes = unit_header_bytes(version)?;
    if unit_count > reader.remaining_len() / unit_header_bytes {
        return Err(PcsQueryPlanSegmentError::LengthOverflow);
    }

    let mut units = Vec::with_capacity(unit_count);
    for _ in 0..unit_count {
        let unit_index = reader.read_u32()?;
        let trace_instance_index = if version == PCS_QUERY_PLAN_V2_VERSION {
            reader.read_u32()?
        } else {
            0
        };
        let query_count = usize::try_from(reader.read_u32()?)
            .map_err(|_| PcsQueryPlanSegmentError::LengthOverflow)?;
        if query_count > reader.remaining_len() / QUERY_BYTES {
            return Err(PcsQueryPlanSegmentError::LengthOverflow);
        }
        let mut queries = Vec::with_capacity(query_count);
        for _ in 0..query_count {
            queries.push(reader.read_u64()?);
        }
        units.push(PcsQueryPlanUnit {
            unit_index,
            trace_instance_index,
            queries,
        });
    }
    reader.finish()?;

    let out = PcsQueryPlanSegment { units };
    validate_pcs_query_plan_segment(&out)?;
    Ok(out)
}

fn query_plan_version(value: &PcsQueryPlanSegment) -> u32 {
    if value
        .units
        .iter()
        .any(|unit| unit.trace_instance_index != 0)
    {
        PCS_QUERY_PLAN_V2_VERSION
    } else {
        PCS_QUERY_PLAN_V1_VERSION
    }
}

fn unit_header_bytes(version: u32) -> Result<usize, PcsQueryPlanSegmentError> {
    match version {
        PCS_QUERY_PLAN_V1_VERSION => Ok(V1_UNIT_HEADER_BYTES),
        PCS_QUERY_PLAN_V2_VERSION => Ok(V2_UNIT_HEADER_BYTES),
        _ => Err(PcsQueryPlanSegmentError::UnsupportedVersion { version }),
    }
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
        if !seen.insert((unit.unit_index, unit.trace_instance_index)) {
            if unit.trace_instance_index == 0 {
                return Err(PcsQueryPlanSegmentError::DuplicateUnitIndex {
                    unit_index: unit.unit_index,
                });
            }
            return Err(PcsQueryPlanSegmentError::DuplicateUnitIdentity {
                unit_index: unit.unit_index,
                trace_instance_index: unit.trace_instance_index,
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

    fn remaining_len(&self) -> usize {
        self.bytes.len() - self.offset
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

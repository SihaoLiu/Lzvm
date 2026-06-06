use std::collections::BTreeSet;
use std::fmt;

pub const TRACE_CONSTRAINT_SEGMENT_ID: u32 = 10_014;

const TRACE_CONSTRAINT_MAGIC: [u8; 4] = *b"tcs0";
const TRACE_CONSTRAINT_VERSION: u32 = 1;
const HEADER_BYTES: usize = 4 + 4 + 4;
const UNIT_BYTES: usize = 4 + 4 + 8 + 4 + 4 + 4;
const FLAG_TRACE_EXTRACTED: u32 = 1 << 0;
const FLAG_REGULAR_CONSTRAINTS_EVALUATED: u32 = 1 << 1;
const FLAG_WITNESS_VALUES_COMMITTED: u32 = 1 << 2;
const FLAG_CONSTRAINT_CHECKER_CONFORMANT: u32 = 1 << 3;
const KNOWN_FLAGS: u32 = FLAG_TRACE_EXTRACTED
    | FLAG_REGULAR_CONSTRAINTS_EVALUATED
    | FLAG_WITNESS_VALUES_COMMITTED
    | FLAG_CONSTRAINT_CHECKER_CONFORMANT;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceConstraintSegment {
    pub units: Vec<TraceConstraintUnitSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceConstraintUnitSegment {
    pub unit_index: u32,
    pub trace_instance_index: u32,
    pub trace_row_count: u64,
    pub trace_column_count: u32,
    pub regular_constraint_count: u32,
    pub trace_extracted: bool,
    pub regular_constraints_evaluated: bool,
    pub witness_values_committed: bool,
    pub constraint_checker_conformant: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceConstraintSegmentError {
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
    EmptyTraceShape {
        unit_index: u32,
    },
    UnknownFlags {
        unit_index: u32,
        flags: u32,
    },
    MissingTraceExtraction {
        unit_index: u32,
    },
    MissingRegularConstraintEvaluation {
        unit_index: u32,
    },
    MissingWitnessCommitment {
        unit_index: u32,
    },
    MissingConstraintCheckerConformance {
        unit_index: u32,
    },
    DuplicateUnitIdentity {
        unit_index: u32,
        trace_instance_index: u32,
    },
    LengthOverflow,
}

impl fmt::Display for TraceConstraintSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid trace constraint segment magic"),
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported trace constraint segment version: {version}")
            }
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "truncated trace constraint segment: needed {needed}, available {available}"
            ),
            Self::TrailingBytes { trailing } => {
                write!(f, "trailing trace constraint segment bytes: {trailing}")
            }
            Self::EmptyUnits => write!(f, "trace constraint segment has no units"),
            Self::EmptyTraceShape { unit_index } => {
                write!(f, "trace constraint unit {unit_index} has an empty trace shape")
            }
            Self::UnknownFlags { unit_index, flags } => write!(
                f,
                "trace constraint unit {unit_index} has unknown flags: {flags}"
            ),
            Self::MissingTraceExtraction { unit_index } => {
                write!(f, "trace constraint unit {unit_index} is missing trace extraction")
            }
            Self::MissingRegularConstraintEvaluation { unit_index } => write!(
                f,
                "trace constraint unit {unit_index} is missing regular constraint evaluation"
            ),
            Self::MissingWitnessCommitment { unit_index } => {
                write!(f, "trace constraint unit {unit_index} is missing witness commitment")
            }
            Self::MissingConstraintCheckerConformance { unit_index } => write!(
                f,
                "trace constraint unit {unit_index} is missing constraint checker conformance"
            ),
            Self::DuplicateUnitIdentity {
                unit_index,
                trace_instance_index,
            } => write!(
                f,
                "duplicate trace constraint unit identity: unit {unit_index}, trace instance {trace_instance_index}"
            ),
            Self::LengthOverflow => write!(f, "trace constraint segment length overflow"),
        }
    }
}

impl std::error::Error for TraceConstraintSegmentError {}

pub fn encode_trace_constraint_segment(
    value: &TraceConstraintSegment,
) -> Result<Vec<u8>, TraceConstraintSegmentError> {
    validate_trace_constraint_segment(value)?;
    let unit_count = u32::try_from(value.units.len())
        .map_err(|_| TraceConstraintSegmentError::LengthOverflow)?;
    let expected_len = HEADER_BYTES
        .checked_add(
            value
                .units
                .len()
                .checked_mul(UNIT_BYTES)
                .ok_or(TraceConstraintSegmentError::LengthOverflow)?,
        )
        .ok_or(TraceConstraintSegmentError::LengthOverflow)?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&TRACE_CONSTRAINT_MAGIC);
    write_u32(&mut out, TRACE_CONSTRAINT_VERSION);
    write_u32(&mut out, unit_count);
    for unit in &value.units {
        write_u32(&mut out, unit.unit_index);
        write_u32(&mut out, unit.trace_instance_index);
        write_u64(&mut out, unit.trace_row_count);
        write_u32(&mut out, unit.trace_column_count);
        write_u32(&mut out, unit.regular_constraint_count);
        write_u32(&mut out, unit_flags(unit));
    }
    Ok(out)
}

pub fn parse_trace_constraint_segment(
    bytes: &[u8],
) -> Result<TraceConstraintSegment, TraceConstraintSegmentError> {
    let mut reader = SegmentReader::new(bytes);
    let magic = reader.read_array::<4>()?;
    if magic != TRACE_CONSTRAINT_MAGIC {
        return Err(TraceConstraintSegmentError::InvalidMagic);
    }
    let version = reader.read_u32()?;
    if version != TRACE_CONSTRAINT_VERSION {
        return Err(TraceConstraintSegmentError::UnsupportedVersion { version });
    }
    let unit_count = usize::try_from(reader.read_u32()?)
        .map_err(|_| TraceConstraintSegmentError::LengthOverflow)?;
    if unit_count == 0 {
        return Err(TraceConstraintSegmentError::EmptyUnits);
    }
    if unit_count > reader.remaining_len() / UNIT_BYTES {
        return Err(TraceConstraintSegmentError::LengthOverflow);
    }

    let mut units = Vec::with_capacity(unit_count);
    for _ in 0..unit_count {
        let unit_index = reader.read_u32()?;
        let trace_instance_index = reader.read_u32()?;
        let trace_row_count = reader.read_u64()?;
        let trace_column_count = reader.read_u32()?;
        let regular_constraint_count = reader.read_u32()?;
        let flags = reader.read_u32()?;
        if flags & !KNOWN_FLAGS != 0 {
            return Err(TraceConstraintSegmentError::UnknownFlags { unit_index, flags });
        }
        units.push(TraceConstraintUnitSegment {
            unit_index,
            trace_instance_index,
            trace_row_count,
            trace_column_count,
            regular_constraint_count,
            trace_extracted: flags & FLAG_TRACE_EXTRACTED != 0,
            regular_constraints_evaluated: flags & FLAG_REGULAR_CONSTRAINTS_EVALUATED != 0,
            witness_values_committed: flags & FLAG_WITNESS_VALUES_COMMITTED != 0,
            constraint_checker_conformant: flags & FLAG_CONSTRAINT_CHECKER_CONFORMANT != 0,
        });
    }
    if reader.remaining_len() != 0 {
        return Err(TraceConstraintSegmentError::TrailingBytes {
            trailing: reader.remaining_len(),
        });
    }

    let out = TraceConstraintSegment { units };
    validate_trace_constraint_segment(&out)?;
    Ok(out)
}

pub fn validate_trace_constraint_segment(
    value: &TraceConstraintSegment,
) -> Result<(), TraceConstraintSegmentError> {
    if value.units.is_empty() {
        return Err(TraceConstraintSegmentError::EmptyUnits);
    }
    let mut seen = BTreeSet::new();
    for unit in &value.units {
        if unit.trace_row_count == 0 || unit.trace_column_count == 0 {
            return Err(TraceConstraintSegmentError::EmptyTraceShape {
                unit_index: unit.unit_index,
            });
        }
        if !unit.trace_extracted {
            return Err(TraceConstraintSegmentError::MissingTraceExtraction {
                unit_index: unit.unit_index,
            });
        }
        if !unit.regular_constraints_evaluated {
            return Err(
                TraceConstraintSegmentError::MissingRegularConstraintEvaluation {
                    unit_index: unit.unit_index,
                },
            );
        }
        if !unit.witness_values_committed {
            return Err(TraceConstraintSegmentError::MissingWitnessCommitment {
                unit_index: unit.unit_index,
            });
        }
        if !unit.constraint_checker_conformant {
            return Err(
                TraceConstraintSegmentError::MissingConstraintCheckerConformance {
                    unit_index: unit.unit_index,
                },
            );
        }
        if !seen.insert((unit.unit_index, unit.trace_instance_index)) {
            return Err(TraceConstraintSegmentError::DuplicateUnitIdentity {
                unit_index: unit.unit_index,
                trace_instance_index: unit.trace_instance_index,
            });
        }
    }
    Ok(())
}

fn unit_flags(unit: &TraceConstraintUnitSegment) -> u32 {
    let mut flags = 0;
    if unit.trace_extracted {
        flags |= FLAG_TRACE_EXTRACTED;
    }
    if unit.regular_constraints_evaluated {
        flags |= FLAG_REGULAR_CONSTRAINTS_EVALUATED;
    }
    if unit.witness_values_committed {
        flags |= FLAG_WITNESS_VALUES_COMMITTED;
    }
    if unit.constraint_checker_conformant {
        flags |= FLAG_CONSTRAINT_CHECKER_CONFORMANT;
    }
    flags
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

    fn remaining_len(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], TraceConstraintSegmentError> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(TraceConstraintSegmentError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(TraceConstraintSegmentError::UnexpectedEof {
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

    fn read_u32(&mut self) -> Result<u32, TraceConstraintSegmentError> {
        Ok(u32::from_le_bytes(self.read_array()?))
    }

    fn read_u64(&mut self) -> Result<u64, TraceConstraintSegmentError> {
        Ok(u64::from_le_bytes(self.read_array()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIRST_UNIT_FLAGS_OFFSET: usize = HEADER_BYTES + 4 + 4 + 8 + 4 + 4;

    fn sample_segment() -> TraceConstraintSegment {
        TraceConstraintSegment {
            units: vec![TraceConstraintUnitSegment {
                unit_index: 0,
                trace_instance_index: 0,
                trace_row_count: 16,
                trace_column_count: 5,
                regular_constraint_count: 1,
                trace_extracted: true,
                regular_constraints_evaluated: true,
                witness_values_committed: true,
                constraint_checker_conformant: true,
            }],
        }
    }

    fn read_first_unit_flags(bytes: &[u8]) -> u32 {
        u32::from_le_bytes(
            bytes[FIRST_UNIT_FLAGS_OFFSET..FIRST_UNIT_FLAGS_OFFSET + 4]
                .try_into()
                .expect("flag bytes should exist"),
        )
    }

    fn write_first_unit_flags(bytes: &mut [u8], flags: u32) {
        bytes[FIRST_UNIT_FLAGS_OFFSET..FIRST_UNIT_FLAGS_OFFSET + 4]
            .copy_from_slice(&flags.to_le_bytes());
    }

    #[test]
    fn trace_constraint_segment_round_trips_required_flags() {
        let segment = sample_segment();
        let encoded = encode_trace_constraint_segment(&segment).expect("segment should encode");
        assert_eq!(parse_trace_constraint_segment(&encoded), Ok(segment));
    }

    #[test]
    fn trace_constraint_segment_rejects_unknown_flags() {
        let mut encoded =
            encode_trace_constraint_segment(&sample_segment()).expect("segment should encode");
        let flags = read_first_unit_flags(&encoded) | (1 << 4);
        write_first_unit_flags(&mut encoded, flags);

        assert_eq!(
            parse_trace_constraint_segment(&encoded),
            Err(TraceConstraintSegmentError::UnknownFlags {
                unit_index: 0,
                flags,
            })
        );
    }

    #[test]
    fn trace_constraint_segment_rejects_missing_conformance_flag() {
        let mut encoded =
            encode_trace_constraint_segment(&sample_segment()).expect("segment should encode");
        let flags = read_first_unit_flags(&encoded) & !FLAG_CONSTRAINT_CHECKER_CONFORMANT;
        write_first_unit_flags(&mut encoded, flags);

        assert_eq!(
            parse_trace_constraint_segment(&encoded),
            Err(TraceConstraintSegmentError::MissingConstraintCheckerConformance { unit_index: 0 })
        );
    }
}

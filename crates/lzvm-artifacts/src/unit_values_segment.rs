use std::collections::BTreeSet;
use std::fmt;

pub const UNIT_VALUES_SEGMENT_ID: u32 = 10_009;

const UNIT_VALUES_MAGIC: [u8; 4] = *b"uvs0";
const UNIT_VALUES_VERSION: u32 = 1;
const HEADER_BYTES: usize = 4 + 4 + 4;
const UNIT_HEADER_BYTES: usize = 4 + 4;
const WORD_BYTES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitValuesSegment {
    pub units: Vec<UnitValuesUnitSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitValuesUnitSegment {
    pub unit_index: u32,
    pub values: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitValuesSegmentError {
    InvalidMagic,
    UnsupportedVersion { version: u32 },
    UnexpectedEof { needed: usize, available: usize },
    TrailingBytes { trailing: usize },
    EmptyUnits,
    EmptyValues { unit_index: u32 },
    DuplicateUnitIndex { unit_index: u32 },
    LengthOverflow,
}

impl fmt::Display for UnitValuesSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid unit values segment magic"),
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported unit values segment version: {version}")
            }
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "truncated unit values segment: needed {needed}, available {available}"
            ),
            Self::TrailingBytes { trailing } => {
                write!(f, "trailing unit values segment bytes: {trailing}")
            }
            Self::EmptyUnits => write!(f, "unit values segment has no units"),
            Self::EmptyValues { unit_index } => {
                write!(f, "unit values unit {unit_index} has no values")
            }
            Self::DuplicateUnitIndex { unit_index } => {
                write!(f, "duplicate unit values unit index: {unit_index}")
            }
            Self::LengthOverflow => write!(f, "unit values segment length overflow"),
        }
    }
}

impl std::error::Error for UnitValuesSegmentError {}

pub fn encode_unit_values_segment(
    value: &UnitValuesSegment,
) -> Result<Vec<u8>, UnitValuesSegmentError> {
    validate_unit_values_segment(value)?;
    let unit_count =
        u32::try_from(value.units.len()).map_err(|_| UnitValuesSegmentError::LengthOverflow)?;
    let expected_len = encoded_len(value)?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&UNIT_VALUES_MAGIC);
    write_u32(&mut out, UNIT_VALUES_VERSION);
    write_u32(&mut out, unit_count);
    for unit in &value.units {
        write_u32(&mut out, unit.unit_index);
        write_u32(
            &mut out,
            u32::try_from(unit.values.len()).map_err(|_| UnitValuesSegmentError::LengthOverflow)?,
        );
        for value in &unit.values {
            write_u64(&mut out, *value);
        }
    }
    Ok(out)
}

pub fn parse_unit_values_segment(
    bytes: &[u8],
) -> Result<UnitValuesSegment, UnitValuesSegmentError> {
    let mut reader = SegmentReader::new(bytes);
    let magic = reader.read_array::<4>()?;
    if magic != UNIT_VALUES_MAGIC {
        return Err(UnitValuesSegmentError::InvalidMagic);
    }
    let version = reader.read_u32()?;
    if version != UNIT_VALUES_VERSION {
        return Err(UnitValuesSegmentError::UnsupportedVersion { version });
    }
    let unit_count = reader.read_u32()? as usize;
    if unit_count == 0 {
        return Err(UnitValuesSegmentError::EmptyUnits);
    }

    let mut units = Vec::with_capacity(unit_count);
    for _ in 0..unit_count {
        let unit_index = reader.read_u32()?;
        let value_count = reader.read_u32()? as usize;
        let mut values = Vec::with_capacity(value_count);
        for _ in 0..value_count {
            values.push(reader.read_u64()?);
        }
        units.push(UnitValuesUnitSegment { unit_index, values });
    }
    reader.finish()?;

    let out = UnitValuesSegment { units };
    validate_unit_values_segment(&out)?;
    Ok(out)
}

fn validate_unit_values_segment(value: &UnitValuesSegment) -> Result<(), UnitValuesSegmentError> {
    if value.units.is_empty() {
        return Err(UnitValuesSegmentError::EmptyUnits);
    }
    let mut seen_units = BTreeSet::new();
    for unit in &value.units {
        if !seen_units.insert(unit.unit_index) {
            return Err(UnitValuesSegmentError::DuplicateUnitIndex {
                unit_index: unit.unit_index,
            });
        }
        if unit.values.is_empty() {
            return Err(UnitValuesSegmentError::EmptyValues {
                unit_index: unit.unit_index,
            });
        }
    }
    Ok(())
}

fn encoded_len(value: &UnitValuesSegment) -> Result<usize, UnitValuesSegmentError> {
    value.units.iter().try_fold(HEADER_BYTES, |acc, unit| {
        let value_bytes = unit
            .values
            .len()
            .checked_mul(WORD_BYTES)
            .ok_or(UnitValuesSegmentError::LengthOverflow)?;
        acc.checked_add(UNIT_HEADER_BYTES)
            .and_then(|bytes| bytes.checked_add(value_bytes))
            .ok_or(UnitValuesSegmentError::LengthOverflow)
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

    fn read_u32(&mut self) -> Result<u32, UnitValuesSegmentError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, UnitValuesSegmentError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], UnitValuesSegmentError> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(UnitValuesSegmentError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(UnitValuesSegmentError::UnexpectedEof {
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

    fn finish(&self) -> Result<(), UnitValuesSegmentError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(UnitValuesSegmentError::TrailingBytes {
                trailing: self.bytes.len() - self.offset,
            })
        }
    }
}

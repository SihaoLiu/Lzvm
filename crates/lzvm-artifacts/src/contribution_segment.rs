use std::collections::BTreeSet;
use std::fmt;

pub const CONTRIBUTION_SEGMENT_ID: u32 = 10_011;

const CONTRIBUTION_MAGIC: [u8; 4] = *b"ctr0";
const CONTRIBUTION_VERSION: u32 = 1;
const HEADER_BYTES: usize = 4 + 4 + 4;
const ENTRY_HEADER_BYTES: usize = 4 + 4 + 4 + 4;
const WORD_BYTES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContributionSegment {
    pub entries: Vec<ContributionEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContributionEntry {
    pub worker_index: u32,
    pub group_id: u32,
    pub aggregated: bool,
    pub values: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContributionSegmentError {
    InvalidMagic,
    UnsupportedVersion { version: u32 },
    UnexpectedEof { needed: usize, available: usize },
    TrailingBytes { trailing: usize },
    EmptyEntries,
    EmptyValues { worker_index: u32, group_id: u32 },
    DuplicateEntry { worker_index: u32, group_id: u32 },
    InvalidAggregatedFlag { value: u32 },
    LengthOverflow,
}

impl fmt::Display for ContributionSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid contribution segment magic"),
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported contribution segment version: {version}")
            }
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "truncated contribution segment: needed {needed}, available {available}"
            ),
            Self::TrailingBytes { trailing } => {
                write!(f, "trailing contribution segment bytes: {trailing}")
            }
            Self::EmptyEntries => write!(f, "contribution segment has no entries"),
            Self::EmptyValues {
                worker_index,
                group_id,
            } => {
                write!(
                    f,
                    "contribution entry worker {worker_index} group {group_id} has no values"
                )
            }
            Self::DuplicateEntry {
                worker_index,
                group_id,
            } => write!(
                f,
                "duplicate contribution entry for worker {worker_index} group {group_id}"
            ),
            Self::InvalidAggregatedFlag { value } => {
                write!(f, "invalid contribution aggregated flag: {value}")
            }
            Self::LengthOverflow => write!(f, "contribution segment length overflow"),
        }
    }
}

impl std::error::Error for ContributionSegmentError {}

pub fn encode_contribution_segment(
    value: &ContributionSegment,
) -> Result<Vec<u8>, ContributionSegmentError> {
    validate_contribution_segment(value)?;
    let entry_count =
        u32::try_from(value.entries.len()).map_err(|_| ContributionSegmentError::LengthOverflow)?;
    let expected_len = encoded_len(value)?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&CONTRIBUTION_MAGIC);
    write_u32(&mut out, CONTRIBUTION_VERSION);
    write_u32(&mut out, entry_count);
    for entry in &value.entries {
        write_u32(&mut out, entry.worker_index);
        write_u32(&mut out, entry.group_id);
        write_u32(&mut out, if entry.aggregated { 1 } else { 0 });
        write_u32(
            &mut out,
            u32::try_from(entry.values.len())
                .map_err(|_| ContributionSegmentError::LengthOverflow)?,
        );
        for value in &entry.values {
            write_u64(&mut out, *value);
        }
    }
    Ok(out)
}

pub fn parse_contribution_segment(
    bytes: &[u8],
) -> Result<ContributionSegment, ContributionSegmentError> {
    let mut reader = SegmentReader::new(bytes);
    let magic = reader.read_array::<4>()?;
    if magic != CONTRIBUTION_MAGIC {
        return Err(ContributionSegmentError::InvalidMagic);
    }
    let version = reader.read_u32()?;
    if version != CONTRIBUTION_VERSION {
        return Err(ContributionSegmentError::UnsupportedVersion { version });
    }
    let entry_count = usize::try_from(reader.read_u32()?)
        .map_err(|_| ContributionSegmentError::LengthOverflow)?;
    if entry_count == 0 {
        return Err(ContributionSegmentError::EmptyEntries);
    }
    if entry_count > reader.remaining_len() / ENTRY_HEADER_BYTES {
        return Err(ContributionSegmentError::LengthOverflow);
    }

    let mut entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let worker_index = reader.read_u32()?;
        let group_id = reader.read_u32()?;
        let aggregated = match reader.read_u32()? {
            0 => false,
            1 => true,
            value => return Err(ContributionSegmentError::InvalidAggregatedFlag { value }),
        };
        let value_count = usize::try_from(reader.read_u32()?)
            .map_err(|_| ContributionSegmentError::LengthOverflow)?;
        if value_count > reader.remaining_len() / WORD_BYTES {
            return Err(ContributionSegmentError::LengthOverflow);
        }
        let mut values = Vec::with_capacity(value_count);
        for _ in 0..value_count {
            values.push(reader.read_u64()?);
        }
        entries.push(ContributionEntry {
            worker_index,
            group_id,
            aggregated,
            values,
        });
    }
    reader.finish()?;

    let out = ContributionSegment { entries };
    validate_contribution_segment(&out)?;
    Ok(out)
}

fn validate_contribution_segment(
    value: &ContributionSegment,
) -> Result<(), ContributionSegmentError> {
    if value.entries.is_empty() {
        return Err(ContributionSegmentError::EmptyEntries);
    }
    let mut seen = BTreeSet::new();
    for entry in &value.entries {
        if !seen.insert((entry.worker_index, entry.group_id)) {
            return Err(ContributionSegmentError::DuplicateEntry {
                worker_index: entry.worker_index,
                group_id: entry.group_id,
            });
        }
        if entry.values.is_empty() {
            return Err(ContributionSegmentError::EmptyValues {
                worker_index: entry.worker_index,
                group_id: entry.group_id,
            });
        }
    }
    Ok(())
}

fn encoded_len(value: &ContributionSegment) -> Result<usize, ContributionSegmentError> {
    value.entries.iter().try_fold(HEADER_BYTES, |acc, entry| {
        let value_bytes = entry
            .values
            .len()
            .checked_mul(WORD_BYTES)
            .ok_or(ContributionSegmentError::LengthOverflow)?;
        acc.checked_add(ENTRY_HEADER_BYTES)
            .and_then(|bytes| bytes.checked_add(value_bytes))
            .ok_or(ContributionSegmentError::LengthOverflow)
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

    fn read_u32(&mut self) -> Result<u32, ContributionSegmentError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, ContributionSegmentError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn remaining_len(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], ContributionSegmentError> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(ContributionSegmentError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(ContributionSegmentError::UnexpectedEof {
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

    fn finish(&self) -> Result<(), ContributionSegmentError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(ContributionSegmentError::TrailingBytes {
                trailing: self.bytes.len() - self.offset,
            })
        }
    }
}

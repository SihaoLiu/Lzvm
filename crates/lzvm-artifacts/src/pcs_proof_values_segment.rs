use std::fmt;

pub const PCS_PROOF_VALUES_SEGMENT_ID: u32 = 10_007;

const PCS_PROOF_VALUES_MAGIC: [u8; 4] = *b"pvs0";
const PCS_PROOF_VALUES_VERSION: u32 = 1;
const HEADER_BYTES: usize = 4 + 4 + 4;
const WORD_BYTES: usize = 8;
const EXTENSION_WORDS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsProofValuesSegment {
    pub values: Vec<[u64; EXTENSION_WORDS]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsProofValuesSegmentError {
    InvalidMagic,
    UnsupportedVersion { version: u32 },
    UnexpectedEof { needed: usize, available: usize },
    TrailingBytes { trailing: usize },
    EmptyValues,
    LengthOverflow,
}

impl fmt::Display for PcsProofValuesSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid PCS proof values segment magic"),
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported PCS proof values segment version: {version}")
            }
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "truncated PCS proof values segment: needed {needed}, available {available}"
            ),
            Self::TrailingBytes { trailing } => {
                write!(f, "trailing PCS proof values segment bytes: {trailing}")
            }
            Self::EmptyValues => write!(f, "PCS proof values segment has no values"),
            Self::LengthOverflow => write!(f, "PCS proof values segment length overflow"),
        }
    }
}

impl std::error::Error for PcsProofValuesSegmentError {}

pub fn encode_pcs_proof_values_segment(
    value: &PcsProofValuesSegment,
) -> Result<Vec<u8>, PcsProofValuesSegmentError> {
    validate_pcs_proof_values_segment(value)?;
    let expected_len = encoded_len(value)?;
    let value_count = u32::try_from(value.values.len())
        .map_err(|_| PcsProofValuesSegmentError::LengthOverflow)?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&PCS_PROOF_VALUES_MAGIC);
    write_u32(&mut out, PCS_PROOF_VALUES_VERSION);
    write_u32(&mut out, value_count);
    for value in &value.values {
        write_extension(&mut out, *value);
    }
    Ok(out)
}

pub fn parse_pcs_proof_values_segment(
    bytes: &[u8],
) -> Result<PcsProofValuesSegment, PcsProofValuesSegmentError> {
    let mut reader = SegmentReader::new(bytes);
    let magic = reader.read_array::<4>()?;
    if magic != PCS_PROOF_VALUES_MAGIC {
        return Err(PcsProofValuesSegmentError::InvalidMagic);
    }
    let version = reader.read_u32()?;
    if version != PCS_PROOF_VALUES_VERSION {
        return Err(PcsProofValuesSegmentError::UnsupportedVersion { version });
    }
    let value_count = reader.read_u32()? as usize;
    let mut values = Vec::with_capacity(value_count);
    for _ in 0..value_count {
        values.push(reader.read_extension()?);
    }
    reader.finish()?;

    let out = PcsProofValuesSegment { values };
    validate_pcs_proof_values_segment(&out)?;
    Ok(out)
}

fn validate_pcs_proof_values_segment(
    value: &PcsProofValuesSegment,
) -> Result<(), PcsProofValuesSegmentError> {
    if value.values.is_empty() {
        return Err(PcsProofValuesSegmentError::EmptyValues);
    }
    Ok(())
}

fn encoded_len(value: &PcsProofValuesSegment) -> Result<usize, PcsProofValuesSegmentError> {
    value
        .values
        .len()
        .checked_mul(EXTENSION_WORDS)
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .and_then(|bytes| bytes.checked_add(HEADER_BYTES))
        .ok_or(PcsProofValuesSegmentError::LengthOverflow)
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

    fn read_u32(&mut self) -> Result<u32, PcsProofValuesSegmentError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, PcsProofValuesSegmentError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn read_extension(&mut self) -> Result<[u64; EXTENSION_WORDS], PcsProofValuesSegmentError> {
        let mut out = [0_u64; EXTENSION_WORDS];
        for word in &mut out {
            *word = self.read_u64()?;
        }
        Ok(out)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], PcsProofValuesSegmentError> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(PcsProofValuesSegmentError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(PcsProofValuesSegmentError::UnexpectedEof {
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

    fn finish(&self) -> Result<(), PcsProofValuesSegmentError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(PcsProofValuesSegmentError::TrailingBytes {
                trailing: self.bytes.len() - self.offset,
            })
        }
    }
}

use std::fmt;

pub const PCS_QUERY_NONCE_SEGMENT_ID: u32 = 10_005;

const PCS_QUERY_NONCE_MAGIC: [u8; 4] = *b"qns0";
const PCS_QUERY_NONCE_VERSION: u32 = 1;
const SEGMENT_BYTES: usize = 4 + 4 + 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsQueryNonceSegment {
    pub nonce: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsQueryNonceSegmentError {
    InvalidMagic,
    UnsupportedVersion { version: u32 },
    UnexpectedEof { needed: usize, available: usize },
    TrailingBytes { trailing: usize },
    LengthOverflow,
}

impl fmt::Display for PcsQueryNonceSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid PCS query nonce segment magic"),
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported PCS query nonce segment version: {version}")
            }
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "truncated PCS query nonce segment: needed {needed}, available {available}"
            ),
            Self::TrailingBytes { trailing } => {
                write!(f, "trailing PCS query nonce segment bytes: {trailing}")
            }
            Self::LengthOverflow => write!(f, "PCS query nonce segment length overflow"),
        }
    }
}

impl std::error::Error for PcsQueryNonceSegmentError {}

pub fn encode_pcs_query_nonce_segment(
    value: &PcsQueryNonceSegment,
) -> Result<Vec<u8>, PcsQueryNonceSegmentError> {
    let mut out = Vec::with_capacity(SEGMENT_BYTES);
    out.extend_from_slice(&PCS_QUERY_NONCE_MAGIC);
    write_u32(&mut out, PCS_QUERY_NONCE_VERSION);
    write_u64(&mut out, value.nonce);
    Ok(out)
}

pub fn parse_pcs_query_nonce_segment(
    bytes: &[u8],
) -> Result<PcsQueryNonceSegment, PcsQueryNonceSegmentError> {
    let mut reader = SegmentReader::new(bytes);
    let magic = reader.read_array::<4>()?;
    if magic != PCS_QUERY_NONCE_MAGIC {
        return Err(PcsQueryNonceSegmentError::InvalidMagic);
    }
    let version = reader.read_u32()?;
    if version != PCS_QUERY_NONCE_VERSION {
        return Err(PcsQueryNonceSegmentError::UnsupportedVersion { version });
    }
    let nonce = reader.read_u64()?;
    reader.finish()?;
    Ok(PcsQueryNonceSegment { nonce })
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

    fn read_u32(&mut self) -> Result<u32, PcsQueryNonceSegmentError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, PcsQueryNonceSegmentError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], PcsQueryNonceSegmentError> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(PcsQueryNonceSegmentError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(PcsQueryNonceSegmentError::UnexpectedEof {
                needed: end,
                available: self.bytes.len(),
            });
        }
        let mut out = [0_u8; N];
        out.copy_from_slice(&self.bytes[self.offset..end]);
        self.offset = end;
        Ok(out)
    }

    fn finish(&self) -> Result<(), PcsQueryNonceSegmentError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(PcsQueryNonceSegmentError::TrailingBytes {
                trailing: self.bytes.len() - self.offset,
            })
        }
    }
}

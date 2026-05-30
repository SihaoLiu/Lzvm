use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramedStdinChunk {
    pub offset: usize,
    pub payload_offset: usize,
    pub payload_len: usize,
    pub padding_len: usize,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FramedStdinError {
    TruncatedLength {
        offset: usize,
        remaining: usize,
    },
    ChunkLengthTooLarge {
        chunk_index: usize,
        len: u64,
    },
    ChunkLengthOverflow {
        chunk_index: usize,
    },
    TruncatedChunk {
        chunk_index: usize,
        expected: usize,
        remaining: usize,
    },
    NonZeroPadding {
        chunk_index: usize,
        offset: usize,
    },
}

pub fn parse_framed_stdin_chunks(bytes: &[u8]) -> Result<Vec<FramedStdinChunk>, FramedStdinError> {
    let mut chunks = Vec::new();
    let mut cursor = 0_usize;
    while cursor < bytes.len() {
        let chunk_index = chunks.len();
        let remaining = bytes.len() - cursor;
        if remaining < 8 {
            return Err(FramedStdinError::TruncatedLength {
                offset: cursor,
                remaining,
            });
        }

        let mut len_bytes = [0_u8; 8];
        len_bytes.copy_from_slice(&bytes[cursor..cursor + 8]);
        let payload_len_u64 = u64::from_le_bytes(len_bytes);
        let payload_len = usize::try_from(payload_len_u64).map_err(|_| {
            FramedStdinError::ChunkLengthTooLarge {
                chunk_index,
                len: payload_len_u64,
            }
        })?;
        let payload_offset = cursor
            .checked_add(8)
            .ok_or(FramedStdinError::ChunkLengthOverflow { chunk_index })?;
        let payload_end = payload_offset
            .checked_add(payload_len)
            .ok_or(FramedStdinError::ChunkLengthOverflow { chunk_index })?;
        let chunk_without_padding = 8_usize
            .checked_add(payload_len)
            .ok_or(FramedStdinError::ChunkLengthOverflow { chunk_index })?;
        let padding_len = (8 - (chunk_without_padding % 8)) % 8;
        let chunk_len = chunk_without_padding
            .checked_add(padding_len)
            .ok_or(FramedStdinError::ChunkLengthOverflow { chunk_index })?;
        let chunk_end = cursor
            .checked_add(chunk_len)
            .ok_or(FramedStdinError::ChunkLengthOverflow { chunk_index })?;
        if chunk_end > bytes.len() {
            return Err(FramedStdinError::TruncatedChunk {
                chunk_index,
                expected: chunk_len,
                remaining,
            });
        }

        for (offset, byte) in bytes[payload_end..chunk_end].iter().copied().enumerate() {
            if byte != 0 {
                return Err(FramedStdinError::NonZeroPadding {
                    chunk_index,
                    offset: payload_end + offset,
                });
            }
        }

        chunks.push(FramedStdinChunk {
            offset: cursor,
            payload_offset,
            payload_len,
            padding_len,
            data: bytes[payload_offset..payload_end].to_vec(),
        });
        cursor = chunk_end;
    }

    Ok(chunks)
}

impl fmt::Display for FramedStdinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedLength { offset, remaining } => write!(
                f,
                "truncated chunk length at offset {offset}: expected 8 bytes, found {remaining}"
            ),
            Self::ChunkLengthTooLarge { chunk_index, len } => {
                write!(f, "chunk {chunk_index} length is too large: {len}")
            }
            Self::ChunkLengthOverflow { chunk_index } => {
                write!(f, "chunk {chunk_index} length overflows")
            }
            Self::TruncatedChunk {
                chunk_index,
                expected,
                remaining,
            } => write!(
                f,
                "truncated chunk {chunk_index}: expected {expected} bytes, found {remaining}"
            ),
            Self::NonZeroPadding {
                chunk_index,
                offset,
            } => write!(
                f,
                "chunk {chunk_index} has nonzero padding byte at offset {offset}"
            ),
        }
    }
}

impl std::error::Error for FramedStdinError {}

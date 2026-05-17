use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RlpItem {
    Bytes(Vec<u8>),
    List(Vec<RlpItem>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RlpError {
    UnexpectedEof {
        offset: usize,
        needed: usize,
        available: usize,
    },
    UnexpectedTrailingBytes {
        offset: usize,
    },
    NonCanonicalSingleByte {
        offset: usize,
    },
    NonCanonicalLength {
        offset: usize,
    },
    LengthOverflow {
        offset: usize,
    },
}

impl fmt::Display for RlpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of RLP at {offset}, needed {needed}, available {available}"
            ),
            Self::UnexpectedTrailingBytes { offset } => {
                write!(f, "unexpected trailing RLP bytes at {offset}")
            }
            Self::NonCanonicalSingleByte { offset } => {
                write!(f, "non-canonical RLP single byte at {offset}")
            }
            Self::NonCanonicalLength { offset } => {
                write!(f, "non-canonical RLP length at {offset}")
            }
            Self::LengthOverflow { offset } => write!(f, "RLP length overflow at {offset}"),
        }
    }
}

impl std::error::Error for RlpError {}

pub fn parse_rlp(bytes: &[u8]) -> Result<RlpItem, RlpError> {
    let (item, next) = parse_item(bytes, 0, bytes.len())?;
    if next != bytes.len() {
        return Err(RlpError::UnexpectedTrailingBytes { offset: next });
    }
    Ok(item)
}

pub fn encode_rlp(item: &RlpItem) -> Vec<u8> {
    match item {
        RlpItem::Bytes(bytes) => encode_bytes(bytes),
        RlpItem::List(items) => {
            let payload = items.iter().flat_map(encode_rlp).collect::<Vec<_>>();
            encode_payload(0xc0, 0xf7, &payload)
        }
    }
}

fn encode_bytes(bytes: &[u8]) -> Vec<u8> {
    if bytes.len() == 1 && bytes[0] <= 0x7f {
        return vec![bytes[0]];
    }
    encode_payload(0x80, 0xb7, bytes)
}

fn encode_payload(short_base: u8, long_base: u8, payload: &[u8]) -> Vec<u8> {
    if payload.len() <= 55 {
        let mut output = vec![short_base + payload.len() as u8];
        output.extend_from_slice(payload);
        return output;
    }

    let length = encode_length(payload.len());
    let mut output = vec![long_base + length.len() as u8];
    output.extend_from_slice(&length);
    output.extend_from_slice(payload);
    output
}

fn encode_length(mut value: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    while value > 0 {
        bytes.push((value & 0xff) as u8);
        value >>= 8;
    }
    bytes.reverse();
    bytes
}

fn parse_item(bytes: &[u8], offset: usize, limit: usize) -> Result<(RlpItem, usize), RlpError> {
    require_available(bytes, offset, limit, 1)?;
    let prefix = bytes[offset];
    match prefix {
        0x00..=0x7f => Ok((RlpItem::Bytes(vec![prefix]), offset + 1)),
        0x80..=0xb7 => parse_short_bytes(bytes, offset, limit, usize::from(prefix - 0x80)),
        0xb8..=0xbf => {
            let length_of_length = usize::from(prefix - 0xb7);
            let (len, payload_start) = parse_long_length(bytes, offset, limit, length_of_length)?;
            parse_bytes_payload(bytes, offset, payload_start, limit, len)
        }
        0xc0..=0xf7 => parse_short_list(bytes, offset, limit, usize::from(prefix - 0xc0)),
        0xf8..=0xff => {
            let length_of_length = usize::from(prefix - 0xf7);
            let (len, payload_start) = parse_long_length(bytes, offset, limit, length_of_length)?;
            parse_list_payload(bytes, payload_start, limit, len)
        }
    }
}

fn parse_short_bytes(
    bytes: &[u8],
    offset: usize,
    limit: usize,
    len: usize,
) -> Result<(RlpItem, usize), RlpError> {
    let payload_start = offset + 1;
    parse_bytes_payload(bytes, offset, payload_start, limit, len)
}

fn parse_bytes_payload(
    bytes: &[u8],
    prefix_offset: usize,
    payload_start: usize,
    limit: usize,
    len: usize,
) -> Result<(RlpItem, usize), RlpError> {
    require_available(bytes, payload_start, limit, len)?;
    let payload_end = checked_add(payload_start, len, prefix_offset)?;
    if len == 1 && bytes[payload_start] <= 0x7f {
        return Err(RlpError::NonCanonicalSingleByte {
            offset: prefix_offset,
        });
    }
    Ok((
        RlpItem::Bytes(bytes[payload_start..payload_end].to_vec()),
        payload_end,
    ))
}

fn parse_short_list(
    bytes: &[u8],
    offset: usize,
    limit: usize,
    len: usize,
) -> Result<(RlpItem, usize), RlpError> {
    parse_list_payload(bytes, offset + 1, limit, len)
}

fn parse_list_payload(
    bytes: &[u8],
    payload_start: usize,
    limit: usize,
    len: usize,
) -> Result<(RlpItem, usize), RlpError> {
    require_available(bytes, payload_start, limit, len)?;
    let payload_end = checked_add(payload_start, len, payload_start)?;
    let mut cursor = payload_start;
    let mut items = Vec::new();
    while cursor < payload_end {
        let (item, next) = parse_item(bytes, cursor, payload_end)?;
        cursor = next;
        items.push(item);
    }
    Ok((RlpItem::List(items), payload_end))
}

fn parse_long_length(
    bytes: &[u8],
    prefix_offset: usize,
    limit: usize,
    length_of_length: usize,
) -> Result<(usize, usize), RlpError> {
    let length_start = prefix_offset + 1;
    require_available(bytes, length_start, limit, length_of_length)?;
    if bytes[length_start] == 0 {
        return Err(RlpError::NonCanonicalLength {
            offset: prefix_offset,
        });
    }
    let length_end = checked_add(length_start, length_of_length, prefix_offset)?;
    let mut len = 0_usize;
    for byte in &bytes[length_start..length_end] {
        len = len
            .checked_mul(256)
            .and_then(|value| value.checked_add(usize::from(*byte)))
            .ok_or(RlpError::LengthOverflow {
                offset: prefix_offset,
            })?;
    }
    if len <= 55 {
        return Err(RlpError::NonCanonicalLength {
            offset: prefix_offset,
        });
    }
    Ok((len, length_end))
}

fn require_available(
    bytes: &[u8],
    offset: usize,
    limit: usize,
    needed: usize,
) -> Result<(), RlpError> {
    let available = limit
        .min(bytes.len())
        .checked_sub(offset)
        .ok_or(RlpError::UnexpectedEof {
            offset,
            needed,
            available: 0,
        })?;
    if available < needed {
        return Err(RlpError::UnexpectedEof {
            offset,
            needed,
            available,
        });
    }
    Ok(())
}

fn checked_add(lhs: usize, rhs: usize, offset: usize) -> Result<usize, RlpError> {
    lhs.checked_add(rhs)
        .ok_or(RlpError::LengthOverflow { offset })
}

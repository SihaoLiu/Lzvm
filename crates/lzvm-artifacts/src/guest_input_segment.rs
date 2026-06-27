use sha2::{Digest, Sha256};

use crate::framed_stdin::{
    parse_framed_stdin_chunks, validate_framed_stdin, FramedStdinChunk, FramedStdinError,
};

pub const FRAMED_GUEST_INPUT_SEGMENT_ID: u32 = 10_015;

pub fn encode_framed_guest_input_segment(bytes: &[u8]) -> Result<Vec<u8>, FramedStdinError> {
    validate_framed_guest_input_segment(bytes)?;
    Ok(bytes.to_vec())
}

pub fn validate_framed_guest_input_segment(bytes: &[u8]) -> Result<(), FramedStdinError> {
    if bytes.is_empty() {
        return Err(FramedStdinError::EmptyInput);
    }
    validate_framed_stdin(bytes)
}

pub fn parse_framed_guest_input_segment(
    bytes: &[u8],
) -> Result<Vec<FramedStdinChunk>, FramedStdinError> {
    if bytes.is_empty() {
        return Err(FramedStdinError::EmptyInput);
    }
    parse_framed_stdin_chunks(bytes)
}

pub fn framed_guest_input_segment_digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

use std::fmt;

use lzvm_field::{Felt, PoseidonTranscript, TranscriptError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsTranscriptError {
    Transcript(TranscriptError),
}

impl fmt::Display for PcsTranscriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transcript(error) => write!(f, "PCS transcript failed: {error}"),
        }
    }
}

impl std::error::Error for PcsTranscriptError {}

impl From<TranscriptError> for PcsTranscriptError {
    fn from(error: TranscriptError) -> Self {
        Self::Transcript(error)
    }
}

pub fn absorb_commit_values(
    transcript: &mut PoseidonTranscript,
    arity: usize,
    hash_values: bool,
    values: &[Felt],
) -> Result<(), PcsTranscriptError> {
    if hash_values {
        let mut inner = PoseidonTranscript::new(arity)?;
        inner.put(values);
        transcript.put(&inner.get_state());
    } else {
        transcript.put(values);
    }
    Ok(())
}

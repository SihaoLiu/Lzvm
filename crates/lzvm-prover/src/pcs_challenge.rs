use std::fmt;

use lzvm_field::{poseidon2_hash_4, Ext3, Felt, PoseidonTranscript, TranscriptError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsChallengeError {
    InvalidWorkBits { bits: u32 },
    QueryNonceNotFound { bits: u32 },
    Transcript(TranscriptError),
}

impl fmt::Display for PcsChallengeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkBits { bits } => {
                write!(f, "invalid PCS challenge work bits: {bits}")
            }
            Self::QueryNonceNotFound { bits } => {
                write!(f, "PCS query nonce search failed for {bits} work bits")
            }
            Self::Transcript(error) => write!(f, "PCS challenge transcript failed: {error}"),
        }
    }
}

impl std::error::Error for PcsChallengeError {}

impl From<TranscriptError> for PcsChallengeError {
    fn from(error: TranscriptError) -> Self {
        Self::Transcript(error)
    }
}

pub fn verify_query_nonce(
    challenge: Ext3,
    nonce: Felt,
    bits: u32,
) -> Result<bool, PcsChallengeError> {
    if bits > 64 {
        return Err(PcsChallengeError::InvalidWorkBits { bits });
    }
    if bits == 0 {
        return Ok(true);
    }
    let digest = poseidon2_hash_4([challenge.c0, challenge.c1, challenge.c2, nonce]);
    let target = if bits == 64 { 1 } else { 1_u64 << (64 - bits) };
    Ok(digest[0].to_u64() < target)
}

pub fn find_query_nonce(challenge: Ext3, bits: u32) -> Result<Felt, PcsChallengeError> {
    if bits > 64 {
        return Err(PcsChallengeError::InvalidWorkBits { bits });
    }
    for candidate in 0..=u64::MAX {
        let nonce = Felt::from_u64(candidate);
        if verify_query_nonce(challenge, nonce, bits)? {
            return Ok(nonce);
        }
    }
    Err(PcsChallengeError::QueryNonceNotFound { bits })
}

pub fn derive_fri_queries(
    arity: usize,
    challenge: Ext3,
    nonce: Felt,
    count: usize,
    bits: u32,
) -> Result<Vec<u64>, PcsChallengeError> {
    let mut transcript = PoseidonTranscript::new(arity)?;
    transcript.put(&[challenge.c0, challenge.c1, challenge.c2]);
    transcript.put(&[nonce]);
    Ok(transcript.get_permutations(count, bits)?)
}

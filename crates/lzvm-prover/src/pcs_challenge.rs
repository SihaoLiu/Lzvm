use std::fmt;

use lzvm_field::{poseidon2_hash_4, Ext3, Felt, PoseidonTranscript, TranscriptError};

#[cfg(feature = "cuda")]
const CUDA_NONCE_BATCH_SIZE: usize = 1 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsChallengeError {
    InvalidWorkBits { bits: u32 },
    QueryNonceNotFound { bits: u32 },
    CudaUnavailable,
    Cuda(String),
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
            Self::CudaUnavailable => write!(f, "PCS query nonce CUDA search is unavailable"),
            Self::Cuda(message) => write!(f, "PCS query nonce CUDA search failed: {message}"),
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

#[cfg(feature = "cuda")]
pub fn find_query_nonce_cuda(challenge: Ext3, bits: u32) -> Result<Felt, PcsChallengeError> {
    find_query_nonce_cuda_with_streams(challenge, bits, 1)
}

#[cfg(feature = "cuda")]
pub fn find_query_nonce_cuda_with_streams(
    challenge: Ext3,
    bits: u32,
    stream_count: usize,
) -> Result<Felt, PcsChallengeError> {
    if bits > 64 {
        return Err(PcsChallengeError::InvalidWorkBits { bits });
    }
    if bits == 0 {
        return Ok(Felt::ZERO);
    }

    let target = if bits == 64 { 1 } else { 1_u64 << (64 - bits) };
    let challenge = [
        challenge.c0.to_u64(),
        challenge.c1.to_u64(),
        challenge.c2.to_u64(),
    ];
    let mut start = 0_u64;
    loop {
        let count = cuda_nonce_batch_len(start, stream_count);
        if let Some(nonce) =
            lzvm_accel::cuda_poseidon2_width4_find_nonce(challenge, start, count, target)
                .map_err(|error| PcsChallengeError::Cuda(error.to_string()))?
        {
            return Ok(Felt::from_u64(nonce));
        }
        if count < CUDA_NONCE_BATCH_SIZE.saturating_mul(stream_count.max(1)) {
            break;
        }
        start = start
            .checked_add((CUDA_NONCE_BATCH_SIZE.saturating_mul(stream_count.max(1))) as u64)
            .ok_or(PcsChallengeError::QueryNonceNotFound { bits })?;
    }
    Err(PcsChallengeError::QueryNonceNotFound { bits })
}

#[cfg(not(feature = "cuda"))]
pub fn find_query_nonce_cuda(_challenge: Ext3, bits: u32) -> Result<Felt, PcsChallengeError> {
    if bits > 64 {
        return Err(PcsChallengeError::InvalidWorkBits { bits });
    }
    Err(PcsChallengeError::CudaUnavailable)
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

#[cfg(feature = "cuda")]
fn cuda_nonce_batch_len(start: u64, stream_count: usize) -> usize {
    let remaining = u64::MAX - start;
    let batch = CUDA_NONCE_BATCH_SIZE.saturating_mul(stream_count.max(1)) as u64;
    if remaining < batch - 1 {
        remaining as usize + 1
    } else {
        CUDA_NONCE_BATCH_SIZE.saturating_mul(stream_count.max(1))
    }
}

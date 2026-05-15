use std::fmt;

use lzvm_field::{Ext3, Felt, PoseidonTranscript, TranscriptError};

#[derive(Debug, Clone, Copy)]
pub struct PcsTranscriptInputs<'a> {
    pub arity: usize,
    pub hash_values: bool,
    pub constant_root: [Felt; 4],
    pub public_values: &'a [Felt],
    pub witness_roots: &'a [[Felt; 4]],
    pub root_challenge_draws: &'a [usize],
    pub evaluation_values: &'a [Ext3],
    pub evaluation_challenge_draws: usize,
    pub fri_roots: &'a [[Felt; 4]],
    pub final_polynomial: &'a [Ext3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsTranscriptError {
    Transcript(TranscriptError),
    RootChallengeDrawMismatch {
        root_count: usize,
        draw_count: usize,
    },
    EmptyFinalPolynomial,
}

impl fmt::Display for PcsTranscriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transcript(error) => write!(f, "PCS transcript failed: {error}"),
            Self::RootChallengeDrawMismatch {
                root_count,
                draw_count,
            } => write!(
                f,
                "PCS transcript root count {root_count} does not match challenge draw count {draw_count}"
            ),
            Self::EmptyFinalPolynomial => write!(f, "PCS transcript final polynomial is empty"),
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

pub fn derive_pcs_final_query_challenge(
    input: PcsTranscriptInputs<'_>,
) -> Result<Ext3, PcsTranscriptError> {
    if input.witness_roots.len() != input.root_challenge_draws.len() {
        return Err(PcsTranscriptError::RootChallengeDrawMismatch {
            root_count: input.witness_roots.len(),
            draw_count: input.root_challenge_draws.len(),
        });
    }
    if input.final_polynomial.is_empty() {
        return Err(PcsTranscriptError::EmptyFinalPolynomial);
    }

    let mut transcript = PoseidonTranscript::new(input.arity)?;
    transcript.put(&input.constant_root);

    if !input.public_values.is_empty() {
        absorb_commit_values(
            &mut transcript,
            input.arity,
            input.hash_values,
            input.public_values,
        )?;
    }

    for (root, draw_count) in input
        .witness_roots
        .iter()
        .zip(input.root_challenge_draws.iter())
    {
        transcript.put(root);
        draw_fields(&mut transcript, *draw_count);
    }

    if !input.evaluation_values.is_empty() {
        let values = flatten_extension_values(input.evaluation_values);
        absorb_commit_values(&mut transcript, input.arity, input.hash_values, &values)?;
    }
    draw_fields(&mut transcript, input.evaluation_challenge_draws);

    for (index, root) in input.fri_roots.iter().enumerate() {
        if index > 0 {
            transcript.get_field();
        }
        transcript.put(root);
    }
    if !input.fri_roots.is_empty() {
        transcript.get_field();
    }

    let final_values = flatten_extension_values(input.final_polynomial);
    absorb_commit_values(
        &mut transcript,
        input.arity,
        input.hash_values,
        &final_values,
    )?;

    Ok(transcript.get_field())
}

fn flatten_extension_values(values: &[Ext3]) -> Vec<Felt> {
    values
        .iter()
        .flat_map(|value| [value.c0, value.c1, value.c2])
        .collect()
}

fn draw_fields(transcript: &mut PoseidonTranscript, count: usize) {
    for _ in 0..count {
        transcript.get_field();
    }
}

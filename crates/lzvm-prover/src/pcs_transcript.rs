use std::fmt;

use lzvm_artifacts::pcs_evaluation_segment::PcsEvaluationUnitSegment;
use lzvm_artifacts::pcs_fri_segment::PcsFriOpeningUnitSegment;
use lzvm_artifacts::pcs_material_segment::PcsMaterialManifestUnit;
use lzvm_artifacts::setup_info::StageValue;
use lzvm_artifacts::witness_segment::WitnessCommitmentSegment;
use lzvm_field::{Ext3, Felt, FieldError, PoseidonTranscript, TranscriptError};

use crate::ProveUnitSchedule;

#[derive(Debug, Clone, Copy)]
pub struct PcsTranscriptInputs<'a> {
    pub arity: usize,
    pub hash_values: bool,
    pub constant_root: [Felt; 4],
    pub public_values: &'a [Felt],
    pub witness_roots: &'a [[Felt; 4]],
    pub root_challenge_draws: &'a [usize],
    pub unit_value_map: &'a [StageValue],
    pub unit_values: &'a [Felt],
    pub evaluation_values: &'a [Ext3],
    pub evaluation_challenge_draws: usize,
    pub fri_roots: &'a [[Felt; 4]],
    pub final_polynomial: &'a [Ext3],
}

#[derive(Debug, Clone, Copy)]
pub struct PcsTranscriptPrefixInputs<'a> {
    pub arity: usize,
    pub hash_values: bool,
    pub constant_root: [Felt; 4],
    pub public_values: &'a [Felt],
    pub witness_roots: &'a [[Felt; 4]],
    pub root_challenge_draws: &'a [usize],
    pub unit_value_map: &'a [StageValue],
    pub unit_values: &'a [Felt],
    pub evaluation_values: &'a [Ext3],
    pub evaluation_challenge_draws: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct PcsTranscriptSegmentInputs<'a> {
    pub unit_index: usize,
    pub unit: &'a ProveUnitSchedule,
    pub material: &'a PcsMaterialManifestUnit,
    pub public_values: &'a [Felt],
    pub unit_values: &'a [Felt],
    pub witness: &'a WitnessCommitmentSegment,
    pub evaluations: &'a PcsEvaluationUnitSegment,
    pub fri: &'a PcsFriOpeningUnitSegment,
    pub root_challenge_draws: &'a [usize],
    pub evaluation_challenge_draws: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsTranscriptError {
    Transcript(TranscriptError),
    Field(FieldError),
    RootChallengeDrawMismatch {
        root_count: usize,
        draw_count: usize,
    },
    UnitIndexOverflow {
        unit_index: usize,
    },
    MissingTranscriptArity {
        unit_index: usize,
    },
    SegmentUnitIndexMismatch {
        segment: &'static str,
        expected: u32,
        found: u32,
    },
    UnitValueOutOfRange {
        value_index: usize,
        offset: usize,
        width: usize,
        len: usize,
    },
    EmptyFinalPolynomial,
    LengthOverflow,
}

impl fmt::Display for PcsTranscriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transcript(error) => write!(f, "PCS transcript failed: {error}"),
            Self::Field(error) => write!(f, "PCS transcript field value failed: {error}"),
            Self::RootChallengeDrawMismatch {
                root_count,
                draw_count,
            } => write!(
                f,
                "PCS transcript root count {root_count} does not match challenge draw count {draw_count}"
            ),
            Self::UnitIndexOverflow { unit_index } => {
                write!(f, "PCS transcript unit index is too large: {unit_index}")
            }
            Self::MissingTranscriptArity { unit_index } => {
                write!(f, "PCS transcript arity is missing for unit {unit_index}")
            }
            Self::SegmentUnitIndexMismatch {
                segment,
                expected,
                found,
            } => write!(
                f,
                "PCS transcript {segment} unit index mismatch: expected {expected}, found {found}"
            ),
            Self::UnitValueOutOfRange {
                value_index,
                offset,
                width,
                len,
            } => write!(
                f,
                "PCS transcript unit value {value_index} offset {offset} with width {width} is outside length {len}"
            ),
            Self::EmptyFinalPolynomial => write!(f, "PCS transcript final polynomial is empty"),
            Self::LengthOverflow => write!(f, "PCS transcript length overflow"),
        }
    }
}

impl std::error::Error for PcsTranscriptError {}

impl From<TranscriptError> for PcsTranscriptError {
    fn from(error: TranscriptError) -> Self {
        Self::Transcript(error)
    }
}

impl From<FieldError> for PcsTranscriptError {
    fn from(error: FieldError) -> Self {
        Self::Field(error)
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
    let challenges = derive_pcs_transcript_challenges(input)?;
    challenges
        .last()
        .copied()
        .ok_or(PcsTranscriptError::EmptyFinalPolynomial)
}

pub fn derive_pcs_transcript_challenges(
    input: PcsTranscriptInputs<'_>,
) -> Result<Vec<Ext3>, PcsTranscriptError> {
    if input.final_polynomial.is_empty() {
        return Err(PcsTranscriptError::EmptyFinalPolynomial);
    }

    let (mut transcript, mut challenges) =
        build_pcs_transcript_prefix(PcsTranscriptPrefixInputs {
            arity: input.arity,
            hash_values: input.hash_values,
            constant_root: input.constant_root,
            public_values: input.public_values,
            witness_roots: input.witness_roots,
            root_challenge_draws: input.root_challenge_draws,
            unit_value_map: input.unit_value_map,
            unit_values: input.unit_values,
            evaluation_values: input.evaluation_values,
            evaluation_challenge_draws: input.evaluation_challenge_draws,
        })?;

    challenges.push(Ext3::ZERO);

    for (index, root) in input.fri_roots.iter().enumerate() {
        if index > 0 {
            challenges.push(transcript.get_field());
        }
        transcript.put(root);
    }
    if !input.fri_roots.is_empty() {
        challenges.push(transcript.get_field());
    }

    let final_values = flatten_extension_values(input.final_polynomial);
    absorb_commit_values(
        &mut transcript,
        input.arity,
        input.hash_values,
        &final_values,
    )?;

    challenges.push(transcript.get_field());
    Ok(challenges)
}

pub fn derive_pcs_transcript_prefix_challenges(
    input: PcsTranscriptPrefixInputs<'_>,
) -> Result<Vec<Ext3>, PcsTranscriptError> {
    let (_, challenges) = build_pcs_transcript_prefix(input)?;
    Ok(challenges)
}

fn build_pcs_transcript_prefix(
    input: PcsTranscriptPrefixInputs<'_>,
) -> Result<(PoseidonTranscript, Vec<Ext3>), PcsTranscriptError> {
    if input.witness_roots.len() != input.root_challenge_draws.len() {
        return Err(PcsTranscriptError::RootChallengeDrawMismatch {
            root_count: input.witness_roots.len(),
            draw_count: input.root_challenge_draws.len(),
        });
    }

    let mut transcript = PoseidonTranscript::new(input.arity)?;
    let mut challenges = Vec::new();
    transcript.put(&input.constant_root);

    if !input.public_values.is_empty() {
        absorb_commit_values(
            &mut transcript,
            input.arity,
            input.hash_values,
            input.public_values,
        )?;
    }

    for (stage_index, (root, draw_count)) in input
        .witness_roots
        .iter()
        .zip(input.root_challenge_draws.iter())
        .enumerate()
    {
        let stage =
            u32::try_from(stage_index + 1).map_err(|_| PcsTranscriptError::LengthOverflow)?;
        transcript.put(root);
        absorb_stage_unit_values(
            &mut transcript,
            stage,
            input.unit_value_map,
            input.unit_values,
        )?;
        draw_fields(&mut transcript, *draw_count, &mut challenges);
    }

    if !input.evaluation_values.is_empty() {
        let values = flatten_extension_values(input.evaluation_values);
        absorb_commit_values(&mut transcript, input.arity, input.hash_values, &values)?;
    }
    draw_fields(
        &mut transcript,
        input.evaluation_challenge_draws,
        &mut challenges,
    );

    Ok((transcript, challenges))
}

pub fn derive_pcs_final_query_challenge_from_segments(
    input: PcsTranscriptSegmentInputs<'_>,
) -> Result<Ext3, PcsTranscriptError> {
    let challenges = derive_pcs_transcript_challenges_from_segments(input)?;
    challenges
        .last()
        .copied()
        .ok_or(PcsTranscriptError::EmptyFinalPolynomial)
}

pub fn derive_pcs_transcript_challenges_from_segments(
    input: PcsTranscriptSegmentInputs<'_>,
) -> Result<Vec<Ext3>, PcsTranscriptError> {
    let expected =
        u32::try_from(input.unit_index).map_err(|_| PcsTranscriptError::UnitIndexOverflow {
            unit_index: input.unit_index,
        })?;
    check_unit_index("material", expected, input.material.unit_index)?;
    check_unit_index("witness", expected, input.witness.unit_index)?;
    check_unit_index("evaluations", expected, input.evaluations.unit_index)?;
    check_unit_index("fri", expected, input.fri.unit_index)?;

    let arity = input
        .unit
        .transcript_arity
        .ok_or(PcsTranscriptError::MissingTranscriptArity {
            unit_index: input.unit_index,
        })? as usize;

    let constant_root = root_from_words(input.material.constant_tree_root)?;
    let witness_roots = input
        .witness
        .stages
        .iter()
        .map(|stage| root_from_words(stage.root))
        .collect::<Result<Vec<_>, _>>()?;
    let evaluation_values = input
        .evaluations
        .values
        .iter()
        .map(|value| extension_from_words(*value))
        .collect::<Result<Vec<_>, _>>()?;
    let fri_roots = input
        .fri
        .layers
        .iter()
        .map(|layer| root_from_words(layer.root))
        .collect::<Result<Vec<_>, _>>()?;
    let final_polynomial = input
        .fri
        .final_polynomial
        .iter()
        .map(|value| extension_from_words(*value))
        .collect::<Result<Vec<_>, _>>()?;

    derive_pcs_transcript_challenges(PcsTranscriptInputs {
        arity,
        hash_values: input.unit.hash_commits,
        constant_root,
        public_values: input.public_values,
        witness_roots: &witness_roots,
        root_challenge_draws: input.root_challenge_draws,
        unit_value_map: &input.unit.unit_value_map,
        unit_values: input.unit_values,
        evaluation_values: &evaluation_values,
        evaluation_challenge_draws: input.evaluation_challenge_draws,
        fri_roots: &fri_roots,
        final_polynomial: &final_polynomial,
    })
}

fn flatten_extension_values(values: &[Ext3]) -> Vec<Felt> {
    values
        .iter()
        .flat_map(|value| [value.c0, value.c1, value.c2])
        .collect()
}

fn absorb_stage_unit_values(
    transcript: &mut PoseidonTranscript,
    stage: u32,
    value_map: &[StageValue],
    values: &[Felt],
) -> Result<(), PcsTranscriptError> {
    let mut offset = 0_usize;
    for (value_index, value) in value_map.iter().enumerate() {
        let width = if value.stage == 1 { 1 } else { 3 };
        let end = offset
            .checked_add(width)
            .ok_or(PcsTranscriptError::LengthOverflow)?;
        if end > values.len() {
            return Err(PcsTranscriptError::UnitValueOutOfRange {
                value_index,
                offset,
                width,
                len: values.len(),
            });
        }
        if value.stage == stage && value.stage > 1 {
            transcript.put(&values[offset..end]);
        }
        offset = end;
    }
    Ok(())
}

fn draw_fields(transcript: &mut PoseidonTranscript, count: usize, out: &mut Vec<Ext3>) {
    for _ in 0..count {
        out.push(transcript.get_field());
    }
}

fn check_unit_index(
    segment: &'static str,
    expected: u32,
    found: u32,
) -> Result<(), PcsTranscriptError> {
    if found == expected {
        Ok(())
    } else {
        Err(PcsTranscriptError::SegmentUnitIndexMismatch {
            segment,
            expected,
            found,
        })
    }
}

fn root_from_words(words: [u64; 4]) -> Result<[Felt; 4], PcsTranscriptError> {
    Ok([
        Felt::from_canonical(words[0])?,
        Felt::from_canonical(words[1])?,
        Felt::from_canonical(words[2])?,
        Felt::from_canonical(words[3])?,
    ])
}

fn extension_from_words(words: [u64; 3]) -> Result<Ext3, PcsTranscriptError> {
    Ok(Ext3::new(
        Felt::from_canonical(words[0])?,
        Felt::from_canonical(words[1])?,
        Felt::from_canonical(words[2])?,
    ))
}

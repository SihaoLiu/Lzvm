use std::collections::BTreeSet;
use std::fmt;

use lzvm_artifacts::contribution_segment::{
    encode_contribution_segment, parse_contribution_segment, ContributionEntry,
    ContributionSegment, ContributionSegmentError, CONTRIBUTION_SEGMENT_ID,
};
use lzvm_artifacts::global_info::{CurveKind, GlobalInfo};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{Ext3, Felt, FieldError, PoseidonTranscript, TranscriptError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveContributionEntry {
    pub worker_index: u32,
    pub group_id: u32,
    pub aggregated: bool,
    pub values: Vec<Felt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveContributionSegmentError {
    Segment(ContributionSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadContributionSegmentError {
    MissingSegment,
    NonCanonicalValue {
        entry_index: usize,
        index: usize,
        source: FieldError,
    },
    Segment(ContributionSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContributionChallengeError {
    UnsupportedCurve {
        curve: CurveKind,
    },
    MissingLatticeSize,
    LatticeSizeOverflow {
        value: u64,
    },
    EmptyEntries,
    EmptyValues {
        entry_index: usize,
    },
    DuplicateEntry {
        worker_index: u32,
        group_id: u32,
    },
    ValueCountMismatch {
        entry_index: usize,
        expected: usize,
        found: usize,
    },
    ProofValueCountMismatch {
        expected: usize,
        found: usize,
    },
    Load(LoadContributionSegmentError),
    LengthOverflow,
    Transcript(TranscriptError),
}

impl fmt::Display for ProveContributionSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Segment(error) => write!(f, "contribution segment encode failed: {error}"),
        }
    }
}

impl std::error::Error for ProveContributionSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
        }
    }
}

impl From<ContributionSegmentError> for ProveContributionSegmentError {
    fn from(error: ContributionSegmentError) -> Self {
        Self::Segment(error)
    }
}

impl fmt::Display for LoadContributionSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing contribution segment"),
            Self::NonCanonicalValue {
                entry_index,
                index,
                source,
            } => write!(
                f,
                "invalid contribution segment entry {entry_index} value {index}: {source}"
            ),
            Self::Segment(error) => write!(f, "invalid contribution segment: {error}"),
        }
    }
}

impl std::error::Error for LoadContributionSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NonCanonicalValue { source, .. } => Some(source),
            Self::Segment(error) => Some(error),
            Self::MissingSegment => None,
        }
    }
}

impl fmt::Display for ContributionChallengeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedCurve { curve } => {
                write!(f, "unsupported contribution curve mode: {curve:?}")
            }
            Self::MissingLatticeSize => write!(f, "missing contribution lattice size"),
            Self::LatticeSizeOverflow { value } => {
                write!(f, "contribution lattice size does not fit usize: {value}")
            }
            Self::EmptyEntries => write!(f, "contribution list has no entries"),
            Self::EmptyValues { entry_index } => {
                write!(f, "contribution entry {entry_index} has no values")
            }
            Self::DuplicateEntry {
                worker_index,
                group_id,
            } => write!(
                f,
                "duplicate contribution entry for worker {worker_index} group {group_id}"
            ),
            Self::ValueCountMismatch {
                entry_index,
                expected,
                found,
            } => write!(
                f,
                "contribution entry {entry_index} value count mismatch: expected {expected}, found {found}"
            ),
            Self::ProofValueCountMismatch { expected, found } => write!(
                f,
                "contribution proof value count mismatch: expected {expected}, found {found}"
            ),
            Self::Load(error) => write!(f, "{error}"),
            Self::LengthOverflow => write!(f, "contribution challenge length overflow"),
            Self::Transcript(error) => write!(f, "contribution challenge transcript failed: {error}"),
        }
    }
}

impl std::error::Error for ContributionChallengeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transcript(error) => Some(error),
            Self::Load(error) => Some(error),
            Self::UnsupportedCurve { .. }
            | Self::MissingLatticeSize
            | Self::LatticeSizeOverflow { .. }
            | Self::EmptyEntries
            | Self::EmptyValues { .. }
            | Self::DuplicateEntry { .. }
            | Self::ValueCountMismatch { .. }
            | Self::ProofValueCountMismatch { .. }
            | Self::LengthOverflow => None,
        }
    }
}

impl From<TranscriptError> for ContributionChallengeError {
    fn from(error: TranscriptError) -> Self {
        Self::Transcript(error)
    }
}

impl From<LoadContributionSegmentError> for ContributionChallengeError {
    fn from(error: LoadContributionSegmentError) -> Self {
        Self::Load(error)
    }
}

pub fn build_contribution_segment(
    entries: &[ProveContributionEntry],
) -> Result<Option<ProofSegment>, ProveContributionSegmentError> {
    if entries.is_empty() {
        return Ok(None);
    }

    let segment = ContributionSegment {
        entries: entries
            .iter()
            .map(|entry| ContributionEntry {
                worker_index: entry.worker_index,
                group_id: entry.group_id,
                aggregated: entry.aggregated,
                values: entry.values.iter().map(|value| value.to_u64()).collect(),
            })
            .collect(),
    };
    Ok(Some(ProofSegment {
        id: CONTRIBUTION_SEGMENT_ID,
        data: encode_contribution_segment(&segment)?,
    }))
}

pub fn load_contribution_segment_from_segments(
    segments: &[ProofSegment],
) -> Result<Vec<ProveContributionEntry>, LoadContributionSegmentError> {
    let segment = segments
        .iter()
        .find(|segment| segment.id == CONTRIBUTION_SEGMENT_ID)
        .ok_or(LoadContributionSegmentError::MissingSegment)?;
    let parsed =
        parse_contribution_segment(&segment.data).map_err(LoadContributionSegmentError::Segment)?;

    parsed
        .entries
        .into_iter()
        .enumerate()
        .map(raw_contribution_entry)
        .collect()
}

pub fn aggregate_contribution_values(
    global_info: &GlobalInfo,
    entries: &[ProveContributionEntry],
) -> Result<Vec<Felt>, ContributionChallengeError> {
    validate_contribution_entries(entries)?;
    match &global_info.curve {
        CurveKind::None => aggregate_lattice_contributions(global_info, entries),
        CurveKind::EcGfp5 | CurveKind::EcMasFp5 => {
            Err(ContributionChallengeError::UnsupportedCurve {
                curve: global_info.curve.clone(),
            })
        }
    }
}

pub fn derive_global_challenge_from_contributions(
    global_info: &GlobalInfo,
    public_values: &[Felt],
    packed_proof_values: &[Felt],
    entries: &[ProveContributionEntry],
) -> Result<Ext3, ContributionChallengeError> {
    let aggregated = aggregate_contribution_values(global_info, entries)?;
    let proof_values = stage_one_proof_values(global_info, packed_proof_values)?;
    let transcript_arity = usize::try_from(global_info.transcript_arity)
        .map_err(|_| ContributionChallengeError::LengthOverflow)?;
    let mut transcript = PoseidonTranscript::new(transcript_arity)?;
    transcript.put(public_values);
    if !proof_values.is_empty() {
        transcript.put(&proof_values);
    }
    transcript.put(&aggregated);
    Ok(transcript.get_field())
}

pub fn derive_global_challenge_from_proof_segments(
    global_info: &GlobalInfo,
    public_values: &[Felt],
    packed_proof_values: &[Felt],
    segments: &[ProofSegment],
) -> Result<Ext3, ContributionChallengeError> {
    let entries = load_contribution_segment_from_segments(segments)?;
    derive_global_challenge_from_contributions(
        global_info,
        public_values,
        packed_proof_values,
        &entries,
    )
}

fn aggregate_lattice_contributions(
    global_info: &GlobalInfo,
    entries: &[ProveContributionEntry],
) -> Result<Vec<Felt>, ContributionChallengeError> {
    let expected = global_info
        .lattice_size
        .ok_or(ContributionChallengeError::MissingLatticeSize)
        .and_then(|value| {
            usize::try_from(value)
                .map_err(|_| ContributionChallengeError::LatticeSizeOverflow { value })
        })?;
    let mut out = vec![Felt::ZERO; expected];
    for (entry_index, entry) in entries.iter().enumerate() {
        if entry.values.len() != expected {
            return Err(ContributionChallengeError::ValueCountMismatch {
                entry_index,
                expected,
                found: entry.values.len(),
            });
        }
        for (index, value) in entry.values.iter().copied().enumerate() {
            out[index] = out[index] + value;
        }
    }
    Ok(out)
}

fn stage_one_proof_values(
    global_info: &GlobalInfo,
    packed_proof_values: &[Felt],
) -> Result<Vec<Felt>, ContributionChallengeError> {
    let expected = expected_packed_proof_value_count(global_info)?;
    if packed_proof_values.len() != expected {
        return Err(ContributionChallengeError::ProofValueCountMismatch {
            expected,
            found: packed_proof_values.len(),
        });
    }

    let mut out = Vec::with_capacity(global_info.stage_one_proof_value_count());
    let mut offset = 0_usize;
    for entry in &global_info.proof_values_map {
        if entry.stage == 1 {
            out.push(packed_proof_values[offset]);
            offset = offset
                .checked_add(1)
                .ok_or(ContributionChallengeError::LengthOverflow)?;
        } else {
            offset = offset
                .checked_add(3)
                .ok_or(ContributionChallengeError::LengthOverflow)?;
        }
    }
    Ok(out)
}

fn expected_packed_proof_value_count(
    global_info: &GlobalInfo,
) -> Result<usize, ContributionChallengeError> {
    global_info
        .proof_values_map
        .iter()
        .try_fold(0_usize, |count, entry| {
            count
                .checked_add(if entry.stage == 1 { 1 } else { 3 })
                .ok_or(ContributionChallengeError::LengthOverflow)
        })
}

fn validate_contribution_entries(
    entries: &[ProveContributionEntry],
) -> Result<(), ContributionChallengeError> {
    if entries.is_empty() {
        return Err(ContributionChallengeError::EmptyEntries);
    }
    let mut seen = BTreeSet::new();
    for (entry_index, entry) in entries.iter().enumerate() {
        if entry.values.is_empty() {
            return Err(ContributionChallengeError::EmptyValues { entry_index });
        }
        if !seen.insert((entry.worker_index, entry.group_id)) {
            return Err(ContributionChallengeError::DuplicateEntry {
                worker_index: entry.worker_index,
                group_id: entry.group_id,
            });
        }
    }
    Ok(())
}

fn raw_contribution_entry(
    (entry_index, entry): (usize, ContributionEntry),
) -> Result<ProveContributionEntry, LoadContributionSegmentError> {
    let values = entry
        .values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            Felt::from_canonical(value).map_err(|source| {
                LoadContributionSegmentError::NonCanonicalValue {
                    entry_index,
                    index,
                    source,
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ProveContributionEntry {
        worker_index: entry.worker_index,
        group_id: entry.group_id,
        aggregated: entry.aggregated,
        values,
    })
}

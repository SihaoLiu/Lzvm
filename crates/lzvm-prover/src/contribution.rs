use std::fmt;

use lzvm_artifacts::contribution_segment::{
    encode_contribution_segment, parse_contribution_segment, ContributionEntry,
    ContributionSegment, ContributionSegmentError, CONTRIBUTION_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{Felt, FieldError};

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

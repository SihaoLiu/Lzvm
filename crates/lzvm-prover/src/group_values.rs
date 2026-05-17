use std::fmt;

use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::group_values_segment::{
    encode_group_values_segment, parse_group_values_segment, GroupValuesSegment,
    GroupValuesSegmentError, GROUP_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{Ext3, Felt, FieldError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveGroupValuesSegmentError {
    UnexpectedValues { found: usize },
    ValueCountMismatch { expected: usize, found: usize },
    LengthOverflow,
    Segment(GroupValuesSegmentError),
}

impl fmt::Display for ProveGroupValuesSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedValues { found } => write!(
                f,
                "group values segment received {found} values but metadata declares none"
            ),
            Self::ValueCountMismatch { expected, found } => write!(
                f,
                "group values segment value count mismatch: expected {expected}, found {found}"
            ),
            Self::LengthOverflow => write!(f, "group values segment length overflow"),
            Self::Segment(error) => write!(f, "group values segment encode failed: {error}"),
        }
    }
}

impl std::error::Error for ProveGroupValuesSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            _ => None,
        }
    }
}

impl From<GroupValuesSegmentError> for ProveGroupValuesSegmentError {
    fn from(error: GroupValuesSegmentError) -> Self {
        Self::Segment(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadGroupValuesSegmentError {
    MissingSegment,
    DuplicateSegment,
    UnexpectedSegment,
    ValueCountMismatch { expected: usize, found: usize },
    NonCanonicalValue { index: usize, source: FieldError },
    Segment(GroupValuesSegmentError),
    LengthOverflow,
}

impl fmt::Display for LoadGroupValuesSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing group values segment"),
            Self::DuplicateSegment => write!(f, "duplicate group values segment"),
            Self::UnexpectedSegment => write!(f, "unexpected group values segment"),
            Self::ValueCountMismatch { expected, found } => write!(
                f,
                "group values segment count mismatch: expected {expected}, found {found}"
            ),
            Self::NonCanonicalValue { index, source } => {
                write!(f, "invalid group values segment value {index}: {source}")
            }
            Self::Segment(error) => write!(f, "invalid group values segment: {error}"),
            Self::LengthOverflow => write!(f, "group values segment length overflow"),
        }
    }
}

impl std::error::Error for LoadGroupValuesSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NonCanonicalValue { source, .. } => Some(source),
            Self::Segment(error) => Some(error),
            Self::MissingSegment
            | Self::DuplicateSegment
            | Self::UnexpectedSegment
            | Self::ValueCountMismatch { .. }
            | Self::LengthOverflow => None,
        }
    }
}

pub fn build_group_values_segment(
    global_info: &GlobalInfo,
    values: &[Ext3],
) -> Result<Option<ProofSegment>, ProveGroupValuesSegmentError> {
    let expected_count = expected_group_value_count(global_info)?;
    if expected_count == 0 {
        if values.is_empty() {
            return Ok(None);
        }
        return Err(ProveGroupValuesSegmentError::UnexpectedValues {
            found: values.len(),
        });
    }
    if values.len() != expected_count {
        return Err(ProveGroupValuesSegmentError::ValueCountMismatch {
            expected: expected_count,
            found: values.len(),
        });
    }

    let segment = GroupValuesSegment {
        values: values.iter().copied().map(Ext3::to_u64s).collect(),
    };
    Ok(Some(ProofSegment {
        id: GROUP_VALUES_SEGMENT_ID,
        data: encode_group_values_segment(&segment)?,
    }))
}

pub fn load_group_values_from_segments(
    global_info: &GlobalInfo,
    segments: &[ProofSegment],
) -> Result<Vec<Ext3>, LoadGroupValuesSegmentError> {
    let expected_count = expected_group_value_count(global_info)
        .map_err(|_| LoadGroupValuesSegmentError::LengthOverflow)?;
    let mut matching_segments = segments
        .iter()
        .filter(|segment| segment.id == GROUP_VALUES_SEGMENT_ID);
    let segment = matching_segments.next();
    if matching_segments.next().is_some() {
        return Err(LoadGroupValuesSegmentError::DuplicateSegment);
    }
    if expected_count == 0 {
        if segment.is_some() {
            return Err(LoadGroupValuesSegmentError::UnexpectedSegment);
        }
        return Ok(Vec::new());
    }

    let segment = segment.ok_or(LoadGroupValuesSegmentError::MissingSegment)?;
    let parsed =
        parse_group_values_segment(&segment.data).map_err(LoadGroupValuesSegmentError::Segment)?;
    if parsed.values.len() != expected_count {
        return Err(LoadGroupValuesSegmentError::ValueCountMismatch {
            expected: expected_count,
            found: parsed.values.len(),
        });
    }

    parsed
        .values
        .iter()
        .copied()
        .enumerate()
        .map(|(index, words)| group_value_extension_from_words(index, words))
        .collect()
}

fn group_value_extension_from_words(
    index: usize,
    words: [u64; 3],
) -> Result<Ext3, LoadGroupValuesSegmentError> {
    Ok(Ext3::new(
        Felt::from_canonical(words[0])
            .map_err(|source| LoadGroupValuesSegmentError::NonCanonicalValue { index, source })?,
        Felt::from_canonical(words[1])
            .map_err(|source| LoadGroupValuesSegmentError::NonCanonicalValue { index, source })?,
        Felt::from_canonical(words[2])
            .map_err(|source| LoadGroupValuesSegmentError::NonCanonicalValue { index, source })?,
    ))
}

fn expected_group_value_count(
    global_info: &GlobalInfo,
) -> Result<usize, ProveGroupValuesSegmentError> {
    global_info
        .aggregation_types
        .iter()
        .try_fold(0_usize, |count, values| {
            count
                .checked_add(values.len())
                .ok_or(ProveGroupValuesSegmentError::LengthOverflow)
        })
}

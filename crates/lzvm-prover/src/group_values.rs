use std::fmt;

use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::group_values_segment::{
    encode_group_values_segment, GroupValuesSegment, GroupValuesSegmentError,
    GROUP_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::Ext3;

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

use std::fmt;

use lzvm_artifacts::global_info::{GlobalInfo, NamedStageValue};
use lzvm_artifacts::pcs_proof_values_segment::{
    encode_pcs_proof_values_segment, parse_pcs_proof_values_segment, PcsProofValuesSegment,
    PcsProofValuesSegmentError, PCS_PROOF_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{Ext3, Felt};

const EXTENSION_WORDS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvePcsProofValuesSegmentError {
    UnexpectedValues { found: usize },
    ValueCountMismatch { expected: usize, found: usize },
    StageOneExtensionComponents { index: usize },
    LengthOverflow,
    Segment(PcsProofValuesSegmentError),
}

impl fmt::Display for ProvePcsProofValuesSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedValues { found } => write!(
                f,
                "proof values segment received {found} values but metadata declares none"
            ),
            Self::ValueCountMismatch { expected, found } => write!(
                f,
                "proof values segment value count mismatch: expected {expected}, found {found}"
            ),
            Self::StageOneExtensionComponents { index } => write!(
                f,
                "proof values segment stage-1 value {index} has nonzero extension components"
            ),
            Self::LengthOverflow => write!(f, "proof values segment length overflow"),
            Self::Segment(error) => write!(f, "proof values segment encode failed: {error}"),
        }
    }
}

impl std::error::Error for ProvePcsProofValuesSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            _ => None,
        }
    }
}

impl From<PcsProofValuesSegmentError> for ProvePcsProofValuesSegmentError {
    fn from(error: PcsProofValuesSegmentError) -> Self {
        Self::Segment(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadPcsProofValuesSegmentError {
    MissingSegment,
    DuplicateSegment,
    UnexpectedSegment,
    ValueCountMismatch { expected: usize, found: usize },
    StageOneExtensionComponents { index: usize },
    Segment(PcsProofValuesSegmentError),
    LengthOverflow,
}

impl fmt::Display for LoadPcsProofValuesSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing PCS proof values segment"),
            Self::DuplicateSegment => write!(f, "duplicate PCS proof values segment"),
            Self::UnexpectedSegment => write!(f, "unexpected PCS proof values segment"),
            Self::ValueCountMismatch { expected, found } => write!(
                f,
                "PCS proof values segment count mismatch: expected {expected}, found {found}"
            ),
            Self::StageOneExtensionComponents { index } => write!(
                f,
                "PCS proof values segment stage-1 value {index} must have zero extension components"
            ),
            Self::Segment(error) => write!(f, "invalid PCS proof values segment: {error}"),
            Self::LengthOverflow => write!(f, "PCS proof values segment length overflow"),
        }
    }
}

impl std::error::Error for LoadPcsProofValuesSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingSegment
            | Self::DuplicateSegment
            | Self::UnexpectedSegment
            | Self::ValueCountMismatch { .. }
            | Self::StageOneExtensionComponents { .. }
            | Self::LengthOverflow => None,
        }
    }
}

pub fn build_pcs_proof_values_segment_from_packed_values(
    global_info: &GlobalInfo,
    packed_values: &[Felt],
) -> Result<Option<ProofSegment>, ProvePcsProofValuesSegmentError> {
    if global_info.proof_values_map.is_empty() {
        if packed_values.is_empty() {
            return Ok(None);
        }
        return Err(ProvePcsProofValuesSegmentError::UnexpectedValues {
            found: packed_values.len(),
        });
    }

    let expected_count = expected_packed_proof_value_count(global_info)?;
    if packed_values.len() != expected_count {
        return Err(ProvePcsProofValuesSegmentError::ValueCountMismatch {
            expected: expected_count,
            found: packed_values.len(),
        });
    }

    let logical_value_count = expected_logical_proof_value_count_for_prove(global_info)?;
    let mut offset = 0_usize;
    let mut values = Vec::with_capacity(logical_value_count);
    for entry in &global_info.proof_values_map {
        for _ in 0..proof_value_dimension_for_prove(entry)? {
            if entry.stage == 1 {
                values.push([packed_values[offset].to_u64(), 0, 0]);
                offset = offset
                    .checked_add(1)
                    .ok_or(ProvePcsProofValuesSegmentError::LengthOverflow)?;
            } else {
                let end = offset
                    .checked_add(EXTENSION_WORDS)
                    .ok_or(ProvePcsProofValuesSegmentError::LengthOverflow)?;
                values.push([
                    packed_values[offset].to_u64(),
                    packed_values[offset + 1].to_u64(),
                    packed_values[offset + 2].to_u64(),
                ]);
                offset = end;
            }
        }
    }

    let segment = PcsProofValuesSegment { values };
    Ok(Some(ProofSegment {
        id: PCS_PROOF_VALUES_SEGMENT_ID,
        data: encode_pcs_proof_values_segment(&segment)?,
    }))
}

pub fn flatten_pcs_proof_values(
    global_info: &GlobalInfo,
    values: &[Ext3],
) -> Result<Vec<Felt>, ProvePcsProofValuesSegmentError> {
    if global_info.proof_values_map.is_empty() {
        if values.is_empty() {
            return Ok(Vec::new());
        }
        return Err(ProvePcsProofValuesSegmentError::UnexpectedValues {
            found: values.len(),
        });
    }

    let expected_value_count = expected_logical_proof_value_count_for_prove(global_info)?;
    if values.len() != expected_value_count {
        return Err(ProvePcsProofValuesSegmentError::ValueCountMismatch {
            expected: expected_value_count,
            found: values.len(),
        });
    }

    let expected_count = expected_packed_proof_value_count(global_info)?;
    let mut packed = Vec::with_capacity(expected_count);
    let mut value_index = 0_usize;
    for entry in &global_info.proof_values_map {
        for _ in 0..proof_value_dimension_for_prove(entry)? {
            let value = values[value_index];
            if entry.stage == 1 {
                if value.c1 != Felt::ZERO || value.c2 != Felt::ZERO {
                    return Err(
                        ProvePcsProofValuesSegmentError::StageOneExtensionComponents {
                            index: value_index,
                        },
                    );
                }
                packed.push(value.c0);
            } else {
                packed.extend_from_slice(&[value.c0, value.c1, value.c2]);
            }
            value_index += 1;
        }
    }
    Ok(packed)
}

pub fn load_pcs_proof_values_from_segments(
    global_info: &GlobalInfo,
    segments: &[ProofSegment],
) -> Result<Vec<Ext3>, LoadPcsProofValuesSegmentError> {
    let expected_count = expected_logical_proof_value_count_for_load(global_info)?;
    let mut matching_segments = segments
        .iter()
        .filter(|segment| segment.id == PCS_PROOF_VALUES_SEGMENT_ID);
    let segment = matching_segments.next();
    if matching_segments.next().is_some() {
        return Err(LoadPcsProofValuesSegmentError::DuplicateSegment);
    }
    if expected_count == 0 {
        if segment.is_some() {
            return Err(LoadPcsProofValuesSegmentError::UnexpectedSegment);
        }
        return Ok(Vec::new());
    }

    let segment = segment.ok_or(LoadPcsProofValuesSegmentError::MissingSegment)?;
    let parsed = parse_pcs_proof_values_segment(&segment.data)
        .map_err(LoadPcsProofValuesSegmentError::Segment)?;
    if parsed.values.len() != expected_count {
        return Err(LoadPcsProofValuesSegmentError::ValueCountMismatch {
            expected: expected_count,
            found: parsed.values.len(),
        });
    }

    let mut values = Vec::with_capacity(parsed.values.len());
    let mut value_index = 0_usize;
    for entry in &global_info.proof_values_map {
        for _ in 0..proof_value_dimension_for_load(entry)? {
            let words = parsed.values[value_index];
            if entry.stage == 1 && (words[1] != 0 || words[2] != 0) {
                return Err(
                    LoadPcsProofValuesSegmentError::StageOneExtensionComponents {
                        index: value_index,
                    },
                );
            }
            values.push(Ext3::from_u64s(words));
            value_index += 1;
        }
    }
    Ok(values)
}

fn expected_packed_proof_value_count(
    global_info: &GlobalInfo,
) -> Result<usize, ProvePcsProofValuesSegmentError> {
    global_info
        .proof_values_map
        .iter()
        .try_fold(0_usize, |count, entry| {
            let dimension = proof_value_dimension_for_prove(entry)?;
            let width = if entry.stage == 1 { 1 } else { EXTENSION_WORDS };
            let entry_count = dimension
                .checked_mul(width)
                .ok_or(ProvePcsProofValuesSegmentError::LengthOverflow)?;
            count
                .checked_add(entry_count)
                .ok_or(ProvePcsProofValuesSegmentError::LengthOverflow)
        })
}

fn expected_logical_proof_value_count_for_prove(
    global_info: &GlobalInfo,
) -> Result<usize, ProvePcsProofValuesSegmentError> {
    global_info
        .proof_values_map
        .iter()
        .try_fold(0_usize, |count, entry| {
            count
                .checked_add(proof_value_dimension_for_prove(entry)?)
                .ok_or(ProvePcsProofValuesSegmentError::LengthOverflow)
        })
}

fn expected_logical_proof_value_count_for_load(
    global_info: &GlobalInfo,
) -> Result<usize, LoadPcsProofValuesSegmentError> {
    global_info
        .proof_values_map
        .iter()
        .try_fold(0_usize, |count, entry| {
            count
                .checked_add(proof_value_dimension_for_load(entry)?)
                .ok_or(LoadPcsProofValuesSegmentError::LengthOverflow)
        })
}

fn proof_value_dimension_for_prove(
    entry: &NamedStageValue,
) -> Result<usize, ProvePcsProofValuesSegmentError> {
    proof_value_dimension(entry).ok_or(ProvePcsProofValuesSegmentError::LengthOverflow)
}

fn proof_value_dimension_for_load(
    entry: &NamedStageValue,
) -> Result<usize, LoadPcsProofValuesSegmentError> {
    proof_value_dimension(entry).ok_or(LoadPcsProofValuesSegmentError::LengthOverflow)
}

fn proof_value_dimension(entry: &NamedStageValue) -> Option<usize> {
    entry.lengths.iter().try_fold(1_usize, |dimension, length| {
        let length = usize::try_from(*length).ok()?;
        dimension.checked_mul(length)
    })
}

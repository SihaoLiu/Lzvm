use std::fmt;

use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::pcs_proof_values_segment::{
    encode_pcs_proof_values_segment, parse_pcs_proof_values_segment, PcsProofValuesSegment,
    PcsProofValuesSegmentError, PCS_PROOF_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{Ext3, Felt, FieldError};

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
    UnexpectedSegment,
    ValueCountMismatch { expected: usize, found: usize },
    StageOneExtensionComponents { index: usize },
    NonCanonicalValue { index: usize, source: FieldError },
    Segment(PcsProofValuesSegmentError),
    LengthOverflow,
}

impl fmt::Display for LoadPcsProofValuesSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing PCS proof values segment"),
            Self::UnexpectedSegment => write!(f, "unexpected PCS proof values segment"),
            Self::ValueCountMismatch { expected, found } => write!(
                f,
                "PCS proof values segment count mismatch: expected {expected}, found {found}"
            ),
            Self::StageOneExtensionComponents { index } => write!(
                f,
                "PCS proof values segment stage-1 value {index} must have zero extension components"
            ),
            Self::NonCanonicalValue { index, source } => {
                write!(
                    f,
                    "invalid PCS proof values segment value {index}: {source}"
                )
            }
            Self::Segment(error) => write!(f, "invalid PCS proof values segment: {error}"),
            Self::LengthOverflow => write!(f, "PCS proof values segment length overflow"),
        }
    }
}

impl std::error::Error for LoadPcsProofValuesSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NonCanonicalValue { source, .. } => Some(source),
            Self::Segment(error) => Some(error),
            Self::MissingSegment
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

    let mut offset = 0_usize;
    let mut values = Vec::with_capacity(global_info.proof_values_map.len());
    for entry in &global_info.proof_values_map {
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

    if values.len() != global_info.proof_values_map.len() {
        return Err(ProvePcsProofValuesSegmentError::ValueCountMismatch {
            expected: global_info.proof_values_map.len(),
            found: values.len(),
        });
    }

    let expected_count = expected_packed_proof_value_count(global_info)?;
    let mut packed = Vec::with_capacity(expected_count);
    for (index, (entry, value)) in global_info
        .proof_values_map
        .iter()
        .zip(values.iter().copied())
        .enumerate()
    {
        if entry.stage == 1 {
            if value.c1 != Felt::ZERO || value.c2 != Felt::ZERO {
                return Err(ProvePcsProofValuesSegmentError::StageOneExtensionComponents { index });
            }
            packed.push(value.c0);
        } else {
            packed.extend_from_slice(&[value.c0, value.c1, value.c2]);
        }
    }
    Ok(packed)
}

pub fn load_pcs_proof_values_from_segments(
    global_info: &GlobalInfo,
    segments: &[ProofSegment],
) -> Result<Vec<Ext3>, LoadPcsProofValuesSegmentError> {
    let expected_count = global_info.proof_values_map.len();
    let segment = segments
        .iter()
        .find(|segment| segment.id == PCS_PROOF_VALUES_SEGMENT_ID);
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
    for (index, words) in parsed.values.iter().copied().enumerate() {
        if global_info.proof_values_map[index].stage == 1 && (words[1] != 0 || words[2] != 0) {
            return Err(LoadPcsProofValuesSegmentError::StageOneExtensionComponents { index });
        }
        values.push(proof_value_extension_from_words(index, words)?);
    }
    Ok(values)
}

fn proof_value_extension_from_words(
    index: usize,
    words: [u64; 3],
) -> Result<Ext3, LoadPcsProofValuesSegmentError> {
    Ok(Ext3::new(
        Felt::from_canonical(words[0]).map_err(|source| {
            LoadPcsProofValuesSegmentError::NonCanonicalValue { index, source }
        })?,
        Felt::from_canonical(words[1]).map_err(|source| {
            LoadPcsProofValuesSegmentError::NonCanonicalValue { index, source }
        })?,
        Felt::from_canonical(words[2]).map_err(|source| {
            LoadPcsProofValuesSegmentError::NonCanonicalValue { index, source }
        })?,
    ))
}

fn expected_packed_proof_value_count(
    global_info: &GlobalInfo,
) -> Result<usize, ProvePcsProofValuesSegmentError> {
    global_info
        .proof_values_map
        .iter()
        .try_fold(0_usize, |count, entry| {
            count
                .checked_add(if entry.stage == 1 { 1 } else { EXTENSION_WORDS })
                .ok_or(ProvePcsProofValuesSegmentError::LengthOverflow)
        })
}

use std::fmt;

use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_opening_segment::{
    parse_witness_opening_segment, WitnessOpeningSegment, WitnessOpeningSegmentError,
    WitnessOpeningUnitSegment, WITNESS_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::witness_segment::{
    parse_witness_commitment_segment, WitnessCommitmentSegmentError,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_field::{Felt, FieldError};

use crate::pcs_query_plan::{load_pcs_query_plan_from_segments, LoadPcsQueryPlanSegmentError};
use crate::witness_commitment::{
    load_witness_commitment_segments, verify_witness_stage_opening_root,
    LoadWitnessCommitmentSegmentsError, WitnessStageOpening, WitnessStageOpeningError,
};
use crate::ProveUnitSchedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadWitnessOpeningSegmentError {
    MissingSegment,
    Segment(WitnessOpeningSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadWitnessOpeningUnitError {
    MissingSegment,
    MissingUnit { unit_index: usize },
    UnitIndexOverflow,
    Segment(WitnessOpeningSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateWitnessOpeningSegmentsError {
    QueryPlan(LoadPcsQueryPlanSegmentError),
    Opening(LoadWitnessOpeningSegmentError),
    Commitments(LoadWitnessCommitmentSegmentsError),
    CommitmentSegment {
        unit_index: usize,
        source: WitnessCommitmentSegmentError,
    },
    StageOpening {
        unit_index: usize,
        source: WitnessStageOpeningError,
    },
    UnitCountMismatch,
    UnitMismatch {
        unit_index: usize,
    },
    UnitIndexOverflow,
    SegmentIdOverflow,
    StageIndexOverflow,
    ArityOverflow,
    InvalidTreeShape,
    LevelCountOverflow,
    FieldValue(FieldError),
    FieldDigest(FieldError),
}

impl fmt::Display for LoadWitnessOpeningSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing witness opening segment"),
            Self::Segment(error) => write!(f, "invalid witness opening segment: {error}"),
        }
    }
}

impl std::error::Error for LoadWitnessOpeningSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingSegment => None,
        }
    }
}

impl fmt::Display for LoadWitnessOpeningUnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing witness opening segment"),
            Self::MissingUnit { unit_index } => {
                write!(f, "witness opening segment mismatch for unit {unit_index}")
            }
            Self::UnitIndexOverflow => write!(f, "witness opening segment unit index overflow"),
            Self::Segment(error) => write!(f, "invalid witness opening segment: {error}"),
        }
    }
}

impl std::error::Error for LoadWitnessOpeningUnitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingSegment | Self::MissingUnit { .. } | Self::UnitIndexOverflow => None,
        }
    }
}

impl From<LoadWitnessOpeningSegmentError> for LoadWitnessOpeningUnitError {
    fn from(error: LoadWitnessOpeningSegmentError) -> Self {
        match error {
            LoadWitnessOpeningSegmentError::MissingSegment => Self::MissingSegment,
            LoadWitnessOpeningSegmentError::Segment(error) => Self::Segment(error),
        }
    }
}

impl fmt::Display for ValidateWitnessOpeningSegmentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueryPlan(error) => write!(f, "{error}"),
            Self::Opening(error) => write!(f, "{error}"),
            Self::Commitments(error) => write!(f, "{error}"),
            Self::CommitmentSegment { unit_index, source } => write!(
                f,
                "invalid witness commitment segment for unit {unit_index}: {source}"
            ),
            Self::StageOpening { unit_index, source } => write!(
                f,
                "invalid witness opening segment for unit {unit_index}: {source}"
            ),
            Self::UnitCountMismatch => write!(f, "witness opening segment unit count mismatch"),
            Self::UnitMismatch { unit_index } => {
                write!(f, "witness opening segment mismatch for unit {unit_index}")
            }
            Self::UnitIndexOverflow => write!(f, "witness opening segment unit index overflow"),
            Self::SegmentIdOverflow => write!(f, "witness opening segment id overflow"),
            Self::StageIndexOverflow => write!(f, "witness opening segment stage index overflow"),
            Self::ArityOverflow => write!(f, "witness opening segment arity overflow"),
            Self::InvalidTreeShape => write!(f, "witness opening segment invalid tree shape"),
            Self::LevelCountOverflow => write!(f, "witness opening segment level count overflow"),
            Self::FieldValue(error) => write!(f, "invalid witness opening segment value: {error}"),
            Self::FieldDigest(error) => {
                write!(f, "invalid witness opening segment digest: {error}")
            }
        }
    }
}

impl std::error::Error for ValidateWitnessOpeningSegmentsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::QueryPlan(error) => Some(error),
            Self::Opening(error) => Some(error),
            Self::Commitments(error) => Some(error),
            Self::CommitmentSegment { source, .. } => Some(source),
            Self::StageOpening { source, .. } => Some(source),
            Self::FieldValue(error) | Self::FieldDigest(error) => Some(error),
            Self::UnitCountMismatch
            | Self::UnitMismatch { .. }
            | Self::UnitIndexOverflow
            | Self::SegmentIdOverflow
            | Self::StageIndexOverflow
            | Self::ArityOverflow
            | Self::InvalidTreeShape
            | Self::LevelCountOverflow => None,
        }
    }
}

pub fn load_witness_opening_segment_from_segments(
    segments: &[ProofSegment],
) -> Result<WitnessOpeningSegment, LoadWitnessOpeningSegmentError> {
    let segment = segments
        .iter()
        .find(|segment| segment.id == WITNESS_OPENING_SEGMENT_ID)
        .ok_or(LoadWitnessOpeningSegmentError::MissingSegment)?;
    parse_witness_opening_segment(&segment.data).map_err(LoadWitnessOpeningSegmentError::Segment)
}

pub fn load_witness_opening_unit_from_segments(
    unit_index: usize,
    segments: &[ProofSegment],
) -> Result<WitnessOpeningUnitSegment, LoadWitnessOpeningUnitError> {
    let opening = load_witness_opening_segment_from_segments(segments)?;
    let unit_index_u32 =
        u32::try_from(unit_index).map_err(|_| LoadWitnessOpeningUnitError::UnitIndexOverflow)?;
    opening
        .units
        .into_iter()
        .find(|unit| unit.unit_index == unit_index_u32)
        .ok_or(LoadWitnessOpeningUnitError::MissingUnit { unit_index })
}

pub fn validate_witness_opening_segments(
    units: &[ProveUnitSchedule],
    segments: &[ProofSegment],
) -> Result<(), ValidateWitnessOpeningSegmentsError> {
    let query_plan = load_pcs_query_plan_from_segments(segments)
        .map_err(ValidateWitnessOpeningSegmentsError::QueryPlan)?;
    let opening = load_witness_opening_segment_from_segments(segments)
        .map_err(ValidateWitnessOpeningSegmentsError::Opening)?;
    if opening.units.len() != query_plan.units.len() {
        return Err(ValidateWitnessOpeningSegmentsError::UnitCountMismatch);
    }

    let witness_segments = load_witness_commitment_segments(units, segments)
        .map_err(ValidateWitnessOpeningSegmentsError::Commitments)?;
    for query_unit in &query_plan.units {
        let unit_index = usize::try_from(query_unit.unit_index)
            .map_err(|_| ValidateWitnessOpeningSegmentsError::UnitIndexOverflow)?;
        let unit = units
            .get(unit_index)
            .ok_or(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index })?;
        let opening_unit = opening
            .units
            .iter()
            .find(|unit| unit.unit_index == query_unit.unit_index)
            .ok_or(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index })?;
        if opening_unit.queries.len() != query_unit.queries.len() {
            return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
        }

        let witness_segment_id = WITNESS_COMMITMENT_SEGMENT_BASE_ID
            .checked_add(query_unit.unit_index)
            .ok_or(ValidateWitnessOpeningSegmentsError::SegmentIdOverflow)?;
        let witness_segment = witness_segments
            .iter()
            .find(|segment| segment.id == witness_segment_id)
            .ok_or(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index })?;
        let witness =
            parse_witness_commitment_segment(&witness_segment.data).map_err(|source| {
                ValidateWitnessOpeningSegmentsError::CommitmentSegment { unit_index, source }
            })?;
        let arity = usize::try_from(unit.merkle_tree_arity)
            .map_err(|_| ValidateWitnessOpeningSegmentsError::ArityOverflow)?;
        let expected_level_count = expected_merkle_level_count(unit.extended_domain_size, arity)?;

        for (query, expected_row) in opening_unit.queries.iter().zip(query_unit.queries.iter()) {
            if query.row_index != *expected_row || query.stages.len() != witness.stages.len() {
                return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
            }
            for stage in &query.stages {
                let stage_index = usize::try_from(stage.stage_index)
                    .map_err(|_| ValidateWitnessOpeningSegmentsError::StageIndexOverflow)?;
                let Some(width) = stage_index
                    .checked_sub(1)
                    .and_then(|index| unit.stage_commit_widths.get(index))
                else {
                    return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
                };
                if stage.values.len() != *width as usize
                    || stage.siblings.len() != expected_level_count
                {
                    return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
                }
                let Some(witness_stage) = witness
                    .stages
                    .iter()
                    .find(|witness_stage| witness_stage.stage_index == stage.stage_index)
                else {
                    return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
                };
                let values = stage
                    .values
                    .iter()
                    .map(|value| Felt::from_canonical(*value))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(ValidateWitnessOpeningSegmentsError::FieldValue)?;
                let siblings = stage
                    .siblings
                    .iter()
                    .map(|level| {
                        if level.siblings.len() + 1 != arity {
                            return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch {
                                unit_index,
                            });
                        }
                        level
                            .siblings
                            .iter()
                            .map(|digest| field_digest_from_words(*digest))
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let opening = WitnessStageOpening::new(query.row_index, values, siblings).map_err(
                    |source| ValidateWitnessOpeningSegmentsError::StageOpening {
                        unit_index,
                        source,
                    },
                )?;
                let root = field_digest_from_words(witness_stage.root)?;
                let stage_arity = usize::try_from(witness_stage.arity)
                    .map_err(|_| ValidateWitnessOpeningSegmentsError::ArityOverflow)?;
                let valid = verify_witness_stage_opening_root(root, stage_arity, &opening)
                    .map_err(|source| ValidateWitnessOpeningSegmentsError::StageOpening {
                        unit_index,
                        source,
                    })?;
                if !valid {
                    return Err(ValidateWitnessOpeningSegmentsError::UnitMismatch { unit_index });
                }
            }
        }
    }
    Ok(())
}

fn expected_merkle_level_count(
    row_count: u64,
    arity: usize,
) -> Result<usize, ValidateWitnessOpeningSegmentsError> {
    if row_count == 0 || arity < 2 {
        return Err(ValidateWitnessOpeningSegmentsError::InvalidTreeShape);
    }
    let arity =
        u64::try_from(arity).map_err(|_| ValidateWitnessOpeningSegmentsError::ArityOverflow)?;
    let mut levels = 0_usize;
    let mut rows = row_count;
    while rows > 1 {
        rows = rows.div_ceil(arity);
        levels = levels
            .checked_add(1)
            .ok_or(ValidateWitnessOpeningSegmentsError::LevelCountOverflow)?;
    }
    Ok(levels)
}

fn field_digest_from_words(
    words: [u64; 4],
) -> Result<[Felt; 4], ValidateWitnessOpeningSegmentsError> {
    let mut out = [Felt::ZERO; 4];
    for (target, value) in out.iter_mut().zip(words) {
        *target = Felt::from_canonical(value)
            .map_err(ValidateWitnessOpeningSegmentsError::FieldDigest)?;
    }
    Ok(out)
}

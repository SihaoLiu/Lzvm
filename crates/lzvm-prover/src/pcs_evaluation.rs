use std::{collections::BTreeSet, fmt};

use lzvm_artifacts::pcs_evaluation_segment::{
    encode_pcs_evaluation_segment, parse_pcs_evaluation_segment, PcsEvaluationSegment,
    PcsEvaluationSegmentError, PcsEvaluationUnitSegment, PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::PcsQueryPlanUnit;
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{Ext3, Felt, FieldError};

use crate::ProveSchedule;
use crate::ProveUnitSchedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvePcsEvaluationValues {
    pub unit_index: usize,
    pub trace_instance_index: u32,
    pub values: Vec<Ext3>,
}

#[derive(Debug, Clone, Copy)]
pub struct ProvePcsEvaluationValueRef<'a> {
    pub unit_index: usize,
    pub trace_instance_index: u32,
    pub values: &'a [Ext3],
}

impl<'a> From<&'a ProvePcsEvaluationValues> for ProvePcsEvaluationValueRef<'a> {
    fn from(values: &'a ProvePcsEvaluationValues) -> Self {
        Self {
            unit_index: values.unit_index,
            trace_instance_index: values.trace_instance_index,
            values: values.values.as_slice(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvePcsEvaluationSegmentError {
    UnitIndexOverflow {
        unit_index: usize,
    },
    UnitIndexOutOfRange {
        unit_index: usize,
        unit_count: usize,
    },
    ValueCountMismatch {
        unit_index: usize,
        expected: usize,
        found: usize,
    },
    Segment(PcsEvaluationSegmentError),
}

impl fmt::Display for ProvePcsEvaluationSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnitIndexOverflow { unit_index } => write!(
                f,
                "prove PCS evaluation segment unit index does not fit u32: {unit_index}"
            ),
            Self::UnitIndexOutOfRange {
                unit_index,
                unit_count,
            } => write!(
                f,
                "prove PCS evaluation segment unit index {unit_index} is outside unit count {unit_count}"
            ),
            Self::ValueCountMismatch {
                unit_index,
                expected,
                found,
            } => write!(
                f,
                "prove PCS evaluation segment unit {unit_index} value count mismatch: expected {expected}, found {found}"
            ),
            Self::Segment(error) => {
                write!(f, "prove PCS evaluation segment encode failed: {error}")
            }
        }
    }
}

impl std::error::Error for ProvePcsEvaluationSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::UnitIndexOverflow { .. }
            | Self::UnitIndexOutOfRange { .. }
            | Self::ValueCountMismatch { .. } => None,
        }
    }
}

impl From<PcsEvaluationSegmentError> for ProvePcsEvaluationSegmentError {
    fn from(error: PcsEvaluationSegmentError) -> Self {
        Self::Segment(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadPcsEvaluationUnitError {
    MissingSegment,
    DuplicateSegment,
    MissingUnit {
        unit_index: usize,
    },
    UnexpectedUnit {
        unit_index: usize,
    },
    UnitIndexOverflow,
    ValueCountMismatch {
        unit_index: usize,
        expected: usize,
        found: usize,
    },
    ValueNonCanonical {
        unit_index: usize,
        value_index: usize,
        word_index: usize,
        source: FieldError,
    },
    Segment(PcsEvaluationSegmentError),
}

impl fmt::Display for LoadPcsEvaluationUnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing PCS evaluation segment"),
            Self::DuplicateSegment => write!(f, "duplicate PCS evaluation segment"),
            Self::MissingUnit { unit_index } => {
                write!(f, "PCS evaluation segment mismatch for unit {unit_index}")
            }
            Self::UnexpectedUnit { unit_index } => {
                write!(f, "unexpected PCS evaluation segment unit {unit_index}")
            }
            Self::UnitIndexOverflow => write!(f, "PCS evaluation segment unit index overflow"),
            Self::ValueCountMismatch { unit_index, .. } => write!(
                f,
                "PCS evaluation segment value count mismatch for unit {unit_index}"
            ),
            Self::ValueNonCanonical {
                unit_index,
                value_index,
                word_index,
                source,
            } => write!(
                f,
                "PCS evaluation segment unit {unit_index} value {value_index} word {word_index} is non-canonical: {source}"
            ),
            Self::Segment(error) => write!(f, "invalid PCS evaluation segment: {error}"),
        }
    }
}

impl std::error::Error for LoadPcsEvaluationUnitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::ValueNonCanonical { source, .. } => Some(source),
            Self::MissingSegment
            | Self::DuplicateSegment
            | Self::MissingUnit { .. }
            | Self::UnexpectedUnit { .. }
            | Self::UnitIndexOverflow
            | Self::ValueCountMismatch { .. } => None,
        }
    }
}

pub fn load_pcs_evaluation_unit_from_segments(
    unit_index: usize,
    unit: &ProveUnitSchedule,
    segments: &[ProofSegment],
) -> Result<PcsEvaluationUnitSegment, LoadPcsEvaluationUnitError> {
    load_pcs_evaluation_unit_for_identity_from_segments(unit_index, 0, unit, segments)
}

pub fn load_pcs_evaluation_unit_for_identity_from_segments(
    unit_index: usize,
    trace_instance_index: u32,
    unit: &ProveUnitSchedule,
    segments: &[ProofSegment],
) -> Result<PcsEvaluationUnitSegment, LoadPcsEvaluationUnitError> {
    let evaluations = load_pcs_evaluation_segment_from_segments(segments)?;
    load_pcs_evaluation_unit_for_identity_from_parsed_segment(
        unit_index,
        trace_instance_index,
        unit,
        &evaluations,
    )
    .cloned()
}

pub(crate) fn load_pcs_evaluation_segment_from_segments(
    segments: &[ProofSegment],
) -> Result<PcsEvaluationSegment, LoadPcsEvaluationUnitError> {
    let mut matching_segments = segments
        .iter()
        .filter(|segment| segment.id == PCS_EVALUATION_SEGMENT_ID);
    let segment = matching_segments
        .next()
        .ok_or(LoadPcsEvaluationUnitError::MissingSegment)?;
    if matching_segments.next().is_some() {
        return Err(LoadPcsEvaluationUnitError::DuplicateSegment);
    }
    parse_pcs_evaluation_segment(&segment.data).map_err(LoadPcsEvaluationUnitError::Segment)
}

pub(crate) fn load_pcs_evaluation_unit_for_identity_from_parsed_segment<'a>(
    unit_index: usize,
    trace_instance_index: u32,
    unit: &ProveUnitSchedule,
    evaluations: &'a PcsEvaluationSegment,
) -> Result<&'a PcsEvaluationUnitSegment, LoadPcsEvaluationUnitError> {
    let unit_index_u32 =
        u32::try_from(unit_index).map_err(|_| LoadPcsEvaluationUnitError::UnitIndexOverflow)?;
    let evaluation_unit = evaluations
        .units
        .iter()
        .find(|unit| {
            unit.unit_index == unit_index_u32 && unit.trace_instance_index == trace_instance_index
        })
        .ok_or(LoadPcsEvaluationUnitError::MissingUnit { unit_index })?;

    let expected_value_count = unit.expected_evaluation_value_count();
    if evaluation_unit.values.len() != expected_value_count {
        return Err(LoadPcsEvaluationUnitError::ValueCountMismatch {
            unit_index,
            expected: expected_value_count,
            found: evaluation_unit.values.len(),
        });
    }
    validate_pcs_evaluation_values(unit_index, &evaluation_unit.values)?;
    Ok(evaluation_unit)
}

fn validate_pcs_evaluation_values(
    unit_index: usize,
    values: &[[u64; 3]],
) -> Result<(), LoadPcsEvaluationUnitError> {
    for (value_index, words) in values.iter().enumerate() {
        for (word_index, word) in words.iter().copied().enumerate() {
            Felt::from_canonical(word).map_err(|source| {
                LoadPcsEvaluationUnitError::ValueNonCanonical {
                    unit_index,
                    value_index,
                    word_index,
                    source,
                }
            })?;
        }
    }
    Ok(())
}

pub(crate) fn validate_pcs_evaluation_units_match_query_units_from_segment(
    query_units: &[PcsQueryPlanUnit],
    evaluations: &PcsEvaluationSegment,
) -> Result<(), LoadPcsEvaluationUnitError> {
    let query_identities = query_units
        .iter()
        .map(|unit| (unit.unit_index, unit.trace_instance_index))
        .collect::<BTreeSet<_>>();
    let mut evaluation_identities = BTreeSet::new();
    for unit in &evaluations.units {
        let identity = (unit.unit_index, unit.trace_instance_index);
        let unit_index = usize::try_from(unit.unit_index)
            .map_err(|_| LoadPcsEvaluationUnitError::UnitIndexOverflow)?;
        if !query_identities.contains(&identity) || !evaluation_identities.insert(identity) {
            return Err(LoadPcsEvaluationUnitError::UnexpectedUnit { unit_index });
        }
    }
    for query_unit in query_units {
        let identity = (query_unit.unit_index, query_unit.trace_instance_index);
        if !evaluation_identities.contains(&identity) {
            let unit_index = usize::try_from(query_unit.unit_index)
                .map_err(|_| LoadPcsEvaluationUnitError::UnitIndexOverflow)?;
            return Err(LoadPcsEvaluationUnitError::MissingUnit { unit_index });
        }
    }
    Ok(())
}

pub fn build_pcs_evaluation_segment(
    schedule: &ProveSchedule,
    values: &[ProvePcsEvaluationValues],
) -> Result<ProofSegment, ProvePcsEvaluationSegmentError> {
    let value_refs = values
        .iter()
        .map(ProvePcsEvaluationValueRef::from)
        .collect::<Vec<_>>();
    build_pcs_evaluation_segment_from_value_refs(schedule, &value_refs)
}

pub fn build_pcs_evaluation_segment_from_value_refs(
    schedule: &ProveSchedule,
    values: &[ProvePcsEvaluationValueRef<'_>],
) -> Result<ProofSegment, ProvePcsEvaluationSegmentError> {
    let mut units = Vec::with_capacity(values.len());
    for input in values {
        let unit = schedule.units.get(input.unit_index).ok_or(
            ProvePcsEvaluationSegmentError::UnitIndexOutOfRange {
                unit_index: input.unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let expected_value_count = unit.expected_evaluation_value_count();
        if input.values.len() != expected_value_count {
            return Err(ProvePcsEvaluationSegmentError::ValueCountMismatch {
                unit_index: input.unit_index,
                expected: expected_value_count,
                found: input.values.len(),
            });
        }
        units.push(PcsEvaluationUnitSegment {
            unit_index: u32::try_from(input.unit_index).map_err(|_| {
                ProvePcsEvaluationSegmentError::UnitIndexOverflow {
                    unit_index: input.unit_index,
                }
            })?,
            trace_instance_index: input.trace_instance_index,
            values: input.values.iter().copied().map(Ext3::to_u64s).collect(),
        });
    }
    units.sort_by_key(|unit| (unit.unit_index, unit.trace_instance_index));

    let segment = PcsEvaluationSegment { units };
    Ok(ProofSegment {
        id: PCS_EVALUATION_SEGMENT_ID,
        data: encode_pcs_evaluation_segment(&segment)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lzvm_artifacts::key_directory::KeyUnitKind;
    use lzvm_field::MODULUS;

    #[test]
    fn parsed_pcs_evaluation_loader_rejects_noncanonical_in_memory_values() {
        let parsed = PcsEvaluationSegment {
            units: vec![PcsEvaluationUnitSegment {
                unit_index: 0,
                trace_instance_index: 0,
                values: vec![[MODULUS, 1, 2]],
            }],
        };

        let error = load_pcs_evaluation_unit_for_identity_from_parsed_segment(
            0,
            0,
            &sample_unit(1),
            &parsed,
        )
        .expect_err("parsed in-memory PCS evaluation value should still be canonical");

        assert_eq!(
            error,
            LoadPcsEvaluationUnitError::ValueNonCanonical {
                unit_index: 0,
                value_index: 0,
                word_index: 0,
                source: FieldError::NonCanonical { value: MODULUS },
            }
        );
    }

    #[test]
    fn evaluation_units_match_query_units_rejects_duplicate_in_memory_identity() {
        let query_units = vec![query_unit(0, 1)];
        let evaluations = PcsEvaluationSegment {
            units: vec![evaluation_unit(0, 1), evaluation_unit(0, 1)],
        };

        let error = validate_pcs_evaluation_units_match_query_units_from_segment(
            &query_units,
            &evaluations,
        )
        .expect_err("duplicate PCS evaluation identity should reject");

        assert_eq!(
            error,
            LoadPcsEvaluationUnitError::UnexpectedUnit { unit_index: 0 }
        );
    }

    fn query_unit(unit_index: u32, trace_instance_index: u32) -> PcsQueryPlanUnit {
        PcsQueryPlanUnit {
            unit_index,
            trace_instance_index,
            queries: vec![0],
        }
    }

    fn evaluation_unit(unit_index: u32, trace_instance_index: u32) -> PcsEvaluationUnitSegment {
        PcsEvaluationUnitSegment {
            unit_index,
            trace_instance_index,
            values: Vec::new(),
        }
    }

    fn sample_unit(evaluation_value_count: usize) -> ProveUnitSchedule {
        ProveUnitSchedule {
            kind: KeyUnitKind::Basic,
            group_id: None,
            unit_id: None,
            group_name: None,
            unit_name: None,
            base_domain_bits: 0,
            extended_domain_bits: 0,
            base_domain_size: 0,
            extended_domain_size: 0,
            blowup_factor: 0,
            query_count: 0,
            proof_of_work_bits: 0,
            merkle_tree_arity: 0,
            last_level_verification: 0,
            transcript_arity: None,
            hash_commits: false,
            transcript_root_challenge_draws: Vec::new(),
            challenge_count: 0,
            evaluation_value_count,
            evaluation_map: Vec::new(),
            transcript_evaluation_challenge_draws: 0,
            constant_width: 0,
            stage_commit_widths: Vec::new(),
            commitment_columns: Vec::new(),
            unit_value_map: Vec::new(),
            group_value_map: Vec::new(),
            opening_points: Vec::new(),
            fri_layers: Vec::new(),
            final_layer_bits: 0,
            fixed_bytes: 0,
            constant_tree_root: None,
            pcs_material_bytes: None,
            pcs_material_plan_digest: None,
            pcs_material_fixed_column_digest: None,
            pcs_material_constant_tree_digest: None,
            pcs_material_constant_tree_root: None,
            pcs_material_fixed_byte_count: None,
            pcs_material_constant_tree_byte_count: None,
            pcs_material_leaf_byte_count: None,
            pcs_material_node_byte_count: None,
        }
    }
}

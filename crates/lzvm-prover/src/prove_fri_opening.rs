mod errors;
mod values;

pub use errors::*;
pub use values::*;

use std::collections::BTreeSet;

use lzvm_artifacts::pcs_evaluation_segment::{
    parse_pcs_evaluation_segment, PcsEvaluationSegment, PcsEvaluationSegmentError,
    PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::pcs_fri_segment::{
    encode_pcs_fri_opening_segment, PcsFriOpeningSegment, PCS_FRI_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::pcs_material_segment::{
    parse_pcs_material_manifest_segment, PcsMaterialManifestSegment,
    PcsMaterialManifestSegmentError, PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::{parse_pcs_query_plan_segment, PCS_QUERY_PLAN_SEGMENT_ID};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_segment::{
    parse_witness_commitment_segment, WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_field::{Ext3, Felt, FieldError};

use crate::pcs_fri::{
    build_pcs_fri_opening_unit, build_pcs_fri_transcript_commitments, PcsFriOpeningBuildRequest,
    PcsFriTranscriptCommitmentRequest,
};
use crate::pcs_query_plan::unsupported_pcs_query_trace_instance;
use crate::pcs_transcript::{derive_pcs_transcript_prefix_challenges, PcsTranscriptPrefixInputs};
use crate::prove_fri_polynomial::{
    build_pcs_fri_polynomial_values, build_pcs_fri_polynomial_values_with_slices,
};
use crate::witness_execution::ProveWitnessAuxiliaryInputSlices;
use crate::ProveSchedule;

type MaterialSegmentParser =
    fn(&[u8]) -> Result<PcsMaterialManifestSegment, PcsMaterialManifestSegmentError>;
type EvaluationSegmentParser = fn(&[u8]) -> Result<PcsEvaluationSegment, PcsEvaluationSegmentError>;

struct MaterialSegmentCache<'a, P = MaterialSegmentParser>
where
    P: FnMut(&[u8]) -> Result<PcsMaterialManifestSegment, PcsMaterialManifestSegmentError>,
{
    entries: Vec<CachedMaterialSegment<'a>>,
    parser: P,
}

struct CachedMaterialSegment<'a> {
    data: &'a [u8],
    parsed: PcsMaterialManifestSegment,
}

impl<'a> MaterialSegmentCache<'a, MaterialSegmentParser> {
    fn new() -> Self {
        Self::with_parser(parse_pcs_material_manifest_segment)
    }
}

impl<'a, P> MaterialSegmentCache<'a, P>
where
    P: FnMut(&[u8]) -> Result<PcsMaterialManifestSegment, PcsMaterialManifestSegmentError>,
{
    fn with_parser(parser: P) -> Self {
        Self {
            entries: Vec::new(),
            parser,
        }
    }

    fn get(
        &mut self,
        segment: &'a ProofSegment,
    ) -> Result<&PcsMaterialManifestSegment, ProvePcsFriTranscriptTraceValuesError> {
        if segment.id != PCS_MATERIAL_MANIFEST_SEGMENT_ID {
            return Err(
                ProvePcsFriTranscriptTraceValuesError::InvalidMaterialSegmentId {
                    segment_id: segment.id,
                },
            );
        }
        if let Some(index) = self
            .entries
            .iter()
            .position(|entry| same_segment_data(entry.data, &segment.data))
        {
            return Ok(&self.entries[index].parsed);
        }
        let parsed = (self.parser)(&segment.data)
            .map_err(ProvePcsFriTranscriptTraceValuesError::MaterialSegment)?;
        self.entries.push(CachedMaterialSegment {
            data: segment.data.as_slice(),
            parsed,
        });
        let index = self.entries.len() - 1;
        Ok(&self.entries[index].parsed)
    }
}

struct EvaluationSegmentCache<'a, P = EvaluationSegmentParser>
where
    P: FnMut(&[u8]) -> Result<PcsEvaluationSegment, PcsEvaluationSegmentError>,
{
    entries: Vec<CachedEvaluationSegment<'a>>,
    parser: P,
}

struct CachedEvaluationSegment<'a> {
    data: &'a [u8],
    parsed: PcsEvaluationSegment,
}

impl<'a> EvaluationSegmentCache<'a, EvaluationSegmentParser> {
    fn new() -> Self {
        Self::with_parser(parse_pcs_evaluation_segment)
    }
}

impl<'a, P> EvaluationSegmentCache<'a, P>
where
    P: FnMut(&[u8]) -> Result<PcsEvaluationSegment, PcsEvaluationSegmentError>,
{
    fn with_parser(parser: P) -> Self {
        Self {
            entries: Vec::new(),
            parser,
        }
    }

    fn get(
        &mut self,
        segment: &'a ProofSegment,
    ) -> Result<&PcsEvaluationSegment, ProvePcsFriTranscriptTraceValuesError> {
        if segment.id != PCS_EVALUATION_SEGMENT_ID {
            return Err(
                ProvePcsFriTranscriptTraceValuesError::InvalidEvaluationSegmentId {
                    segment_id: segment.id,
                },
            );
        }
        if let Some(index) = self
            .entries
            .iter()
            .position(|entry| same_segment_data(entry.data, &segment.data))
        {
            return Ok(&self.entries[index].parsed);
        }
        let parsed = (self.parser)(&segment.data)
            .map_err(ProvePcsFriTranscriptTraceValuesError::EvaluationSegment)?;
        self.entries.push(CachedEvaluationSegment {
            data: segment.data.as_slice(),
            parsed,
        });
        let index = self.entries.len() - 1;
        Ok(&self.entries[index].parsed)
    }
}

fn same_segment_data(left: &[u8], right: &[u8]) -> bool {
    (left.len() == right.len() && std::ptr::eq(left.as_ptr(), right.as_ptr())) || left == right
}

#[derive(Debug, Clone, Copy)]
struct PcsFriOpeningValueRef<'a> {
    unit_index: usize,
    challenges: &'a [Ext3],
    polynomial: &'a [Ext3],
}

struct PcsFriOpeningTraceValue<'a> {
    unit_index: usize,
    challenges: &'a [Ext3],
    polynomial: Vec<Ext3>,
}

pub fn build_pcs_fri_opening_segment(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    values: &[ProvePcsFriOpeningValues],
) -> Result<ProofSegment, ProvePcsFriOpeningSegmentError> {
    build_pcs_fri_opening_segment_from_value_refs(
        schedule,
        query_segment,
        values.iter().map(|value| PcsFriOpeningValueRef {
            unit_index: value.unit_index,
            challenges: &value.challenges,
            polynomial: &value.polynomial,
        }),
        values.len(),
    )
}

fn build_pcs_fri_opening_segment_from_value_refs<'a>(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    values: impl IntoIterator<Item = PcsFriOpeningValueRef<'a>>,
    value_count: usize,
) -> Result<ProofSegment, ProvePcsFriOpeningSegmentError> {
    if query_segment.id != PCS_QUERY_PLAN_SEGMENT_ID {
        return Err(ProvePcsFriOpeningSegmentError::InvalidQuerySegmentId {
            segment_id: query_segment.id,
        });
    }
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
    if let Some(unsupported) = unsupported_pcs_query_trace_instance(&query_plan.units) {
        return Err(ProvePcsFriOpeningSegmentError::UnsupportedTraceInstance {
            unit_index: unsupported.unit_index,
            trace_instance_index: unsupported.trace_instance_index,
        });
    }
    let mut seen_units = BTreeSet::new();
    let mut units = Vec::with_capacity(value_count);
    for input in values {
        if !seen_units.insert(input.unit_index) {
            return Err(ProvePcsFriOpeningSegmentError::DuplicateUnitIndex {
                unit_index: input.unit_index,
            });
        }
        let unit = schedule.units.get(input.unit_index).ok_or(
            ProvePcsFriOpeningSegmentError::UnitIndexOutOfRange {
                unit_index: input.unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let unit_index_u32 = u32::try_from(input.unit_index).map_err(|_| {
            ProvePcsFriOpeningSegmentError::UnitIndexOverflow {
                unit_index: input.unit_index,
            }
        })?;
        let query_unit = query_plan
            .units
            .iter()
            .find(|unit| unit.unit_index == unit_index_u32)
            .ok_or(ProvePcsFriOpeningSegmentError::MissingQueryUnit {
                unit_index: input.unit_index,
            })?;
        let opening = build_pcs_fri_opening_unit(
            unit,
            PcsFriOpeningBuildRequest {
                unit_index: unit_index_u32,
                query_rows: &query_unit.queries,
                challenges: input.challenges,
                polynomial: input.polynomial,
            },
        )
        .map_err(|source| ProvePcsFriOpeningSegmentError::Build {
            unit_index: input.unit_index,
            source,
        })?;
        units.push(opening);
    }

    let segment = PcsFriOpeningSegment { units };
    Ok(ProofSegment {
        id: PCS_FRI_OPENING_SEGMENT_ID,
        data: encode_pcs_fri_opening_segment(&segment)?,
    })
}

pub fn build_pcs_fri_transcript_values_from_trace(
    schedule: &ProveSchedule,
    values: &[ProvePcsFriTranscriptTraceValues<'_>],
) -> Result<Vec<ProvePcsFriTranscriptValues>, ProvePcsFriTranscriptTraceValuesError> {
    let refs = values
        .iter()
        .map(|input| ProvePcsFriTranscriptTraceValueRef {
            unit_index: input.unit_index,
            execution_unit: input.execution_unit,
            trace: input.trace,
            publics: input.publics,
            auxiliary_inputs: ProveWitnessAuxiliaryInputSlices::from(input.auxiliary_inputs),
            constant_root: input.constant_root,
            witness_roots: input.witness_roots,
            evaluation_values: input.evaluation_values,
            xi_challenge: input.xi_challenge,
            binding_segments: input.binding_segments,
        })
        .collect::<Vec<_>>();
    build_pcs_fri_transcript_values_from_trace_refs(schedule, &refs)
}

fn build_pcs_fri_transcript_values_from_trace_refs(
    schedule: &ProveSchedule,
    values: &[ProvePcsFriTranscriptTraceValueRef<'_>],
) -> Result<Vec<ProvePcsFriTranscriptValues>, ProvePcsFriTranscriptTraceValuesError> {
    let mut out = Vec::with_capacity(values.len());
    for input in values {
        let unit = schedule.units.get(input.unit_index).ok_or(
            ProvePcsFriTranscriptTraceValuesError::UnitIndexOutOfRange {
                unit_index: input.unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let arity = unit.transcript_arity.ok_or(
            ProvePcsFriTranscriptTraceValuesError::MissingTranscriptArity {
                unit_index: input.unit_index,
            },
        )? as usize;
        let polynomial = build_pcs_fri_polynomial_values_with_slices(
            input.unit_index,
            unit,
            input.execution_unit,
            input.trace,
            input.publics,
            input.auxiliary_inputs,
            input.xi_challenge,
        )
        .map_err(|source| ProvePcsFriTranscriptTraceValuesError::Polynomial {
            unit_index: input.unit_index,
            source: Box::new(source),
        })?;
        let commitments = build_pcs_fri_transcript_commitments(
            unit,
            PcsFriTranscriptCommitmentRequest {
                arity,
                hash_values: unit.hash_commits,
                constant_root: input.constant_root,
                public_values: input.publics,
                witness_roots: input.witness_roots,
                root_challenge_draws: &unit.transcript_root_challenge_draws,
                unit_value_map: &unit.unit_value_map,
                unit_values: input.auxiliary_inputs.unit_values,
                evaluation_values: input.evaluation_values,
                evaluation_challenge_draws: unit.transcript_evaluation_challenge_draws,
                polynomial: &polynomial,
                binding_segments: input.binding_segments,
            },
        )
        .map_err(|source| ProvePcsFriTranscriptTraceValuesError::Transcript {
            unit_index: input.unit_index,
            source: Box::new(source),
        })?;
        out.push(ProvePcsFriTranscriptValues {
            unit_index: input.unit_index,
            polynomial,
            commitments,
        });
    }
    Ok(out)
}

pub fn build_pcs_fri_transcript_values_from_trace_segments(
    schedule: &ProveSchedule,
    values: &[ProvePcsFriTranscriptTraceSegmentValues<'_>],
) -> Result<Vec<ProvePcsFriTranscriptValues>, ProvePcsFriTranscriptTraceValuesError> {
    let refs = values
        .iter()
        .map(|input| ProvePcsFriTranscriptTraceSegmentValueRef {
            unit_index: input.unit_index,
            execution_unit: input.execution_unit,
            trace: input.trace,
            publics: input.publics,
            auxiliary_inputs: ProveWitnessAuxiliaryInputSlices::from(input.auxiliary_inputs),
            material_segment: input.material_segment,
            witness_segment: input.witness_segment,
            evaluation_segment: input.evaluation_segment,
            binding_segments: input.binding_segments,
        })
        .collect::<Vec<_>>();
    build_pcs_fri_transcript_values_from_trace_segment_refs(schedule, &refs)
}

pub(crate) fn build_pcs_fri_transcript_values_from_trace_segment_refs(
    schedule: &ProveSchedule,
    values: &[ProvePcsFriTranscriptTraceSegmentValueRef<'_>],
) -> Result<Vec<ProvePcsFriTranscriptValues>, ProvePcsFriTranscriptTraceValuesError> {
    let mut out = Vec::with_capacity(values.len());
    let mut material_cache = MaterialSegmentCache::new();
    let mut evaluation_cache = EvaluationSegmentCache::new();
    for input in values {
        let unit = schedule.units.get(input.unit_index).ok_or(
            ProvePcsFriTranscriptTraceValuesError::UnitIndexOutOfRange {
                unit_index: input.unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let unit_index_u32 = u32::try_from(input.unit_index).map_err(|_| {
            ProvePcsFriTranscriptTraceValuesError::UnitIndexOverflow {
                unit_index: input.unit_index,
            }
        })?;
        let arity = unit.transcript_arity.ok_or(
            ProvePcsFriTranscriptTraceValuesError::MissingTranscriptArity {
                unit_index: input.unit_index,
            },
        )? as usize;
        validate_material_segment_id(input.material_segment)?;
        let expected_witness_id = WITNESS_COMMITMENT_SEGMENT_BASE_ID
            .checked_add(unit_index_u32)
            .ok_or(ProvePcsFriTranscriptTraceValuesError::UnitIndexOverflow {
                unit_index: input.unit_index,
            })?;
        if input.witness_segment.id != expected_witness_id {
            return Err(
                ProvePcsFriTranscriptTraceValuesError::InvalidWitnessSegmentId {
                    unit_index: input.unit_index,
                    expected: expected_witness_id,
                    found: input.witness_segment.id,
                },
            );
        }
        validate_evaluation_segment_id(input.evaluation_segment)?;

        let material = material_cache
            .get(input.material_segment)?
            .units
            .iter()
            .find(|unit| unit.unit_index == unit_index_u32)
            .ok_or(ProvePcsFriTranscriptTraceValuesError::MissingMaterialUnit {
                unit_index: input.unit_index,
            })?;
        let witness =
            parse_witness_commitment_segment(&input.witness_segment.data).map_err(|source| {
                ProvePcsFriTranscriptTraceValuesError::WitnessSegment {
                    unit_index: input.unit_index,
                    source,
                }
            })?;
        if witness.unit_index != unit_index_u32 {
            return Err(
                ProvePcsFriTranscriptTraceValuesError::SegmentUnitIndexMismatch {
                    segment: "witness",
                    expected: unit_index_u32,
                    found: witness.unit_index,
                },
            );
        }
        let evaluations = evaluation_cache
            .get(input.evaluation_segment)?
            .units
            .iter()
            .find(|unit| unit.unit_index == unit_index_u32)
            .ok_or(
                ProvePcsFriTranscriptTraceValuesError::MissingEvaluationUnit {
                    unit_index: input.unit_index,
                },
            )?;

        let constant_root = root_from_words(material.constant_tree_root).map_err(|source| {
            ProvePcsFriTranscriptTraceValuesError::Field {
                unit_index: input.unit_index,
                source,
            }
        })?;
        let witness_roots = witness
            .stages
            .iter()
            .map(|stage| root_from_words(stage.root))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| ProvePcsFriTranscriptTraceValuesError::Field {
                unit_index: input.unit_index,
                source,
            })?;
        let evaluation_values = evaluations
            .values
            .iter()
            .map(|value| extension_from_words(*value))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| ProvePcsFriTranscriptTraceValuesError::Field {
                unit_index: input.unit_index,
                source,
            })?;
        let prefix_challenges =
            derive_pcs_transcript_prefix_challenges(PcsTranscriptPrefixInputs {
                arity,
                hash_values: unit.hash_commits,
                constant_root,
                public_values: input.publics,
                witness_roots: &witness_roots,
                root_challenge_draws: &unit.transcript_root_challenge_draws,
                unit_value_map: &unit.unit_value_map,
                unit_values: input.auxiliary_inputs.unit_values,
                evaluation_values: &evaluation_values,
                evaluation_challenge_draws: unit.transcript_evaluation_challenge_draws,
                binding_segments: input.binding_segments,
            })
            .map_err(|source| {
                ProvePcsFriTranscriptTraceValuesError::PrefixTranscript {
                    unit_index: input.unit_index,
                    source: Box::new(source),
                }
            })?;
        let xi_index = unit.challenge_count.checked_sub(3).ok_or(
            ProvePcsFriTranscriptTraceValuesError::MissingXiChallenge {
                unit_index: input.unit_index,
                challenge_count: unit.challenge_count,
            },
        )?;
        let xi_challenge = *prefix_challenges.get(xi_index).ok_or(
            ProvePcsFriTranscriptTraceValuesError::PrefixChallengeOutOfRange {
                unit_index: input.unit_index,
                index: xi_index,
                len: prefix_challenges.len(),
            },
        )?;

        let mut built = build_pcs_fri_transcript_values_from_trace_refs(
            schedule,
            &[ProvePcsFriTranscriptTraceValueRef {
                unit_index: input.unit_index,
                execution_unit: input.execution_unit,
                trace: input.trace,
                publics: input.publics,
                auxiliary_inputs: input.auxiliary_inputs,
                constant_root,
                witness_roots: &witness_roots,
                evaluation_values: &evaluation_values,
                xi_challenge,
                binding_segments: input.binding_segments,
            }],
        )?;
        out.append(&mut built);
    }
    Ok(out)
}

pub fn build_pcs_fri_opening_segment_from_transcript_values(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    values: &[ProvePcsFriTranscriptValues],
) -> Result<ProofSegment, ProvePcsFriOpeningSegmentError> {
    build_pcs_fri_opening_segment_from_value_refs(
        schedule,
        query_segment,
        values.iter().map(|value| PcsFriOpeningValueRef {
            unit_index: value.unit_index,
            challenges: &value.commitments.challenges,
            polynomial: &value.polynomial,
        }),
        values.len(),
    )
}

pub fn build_pcs_fri_opening_segment_from_trace(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    values: &[ProvePcsFriOpeningTraceValues<'_>],
) -> Result<ProofSegment, ProvePcsFriOpeningTraceSegmentError> {
    let mut opening_values = Vec::with_capacity(values.len());
    for input in values {
        let unit = schedule.units.get(input.unit_index).ok_or(
            ProvePcsFriOpeningTraceSegmentError::UnitIndexOutOfRange {
                unit_index: input.unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let polynomial = build_pcs_fri_polynomial_values(
            input.unit_index,
            unit,
            input.execution_unit,
            input.trace,
            input.publics,
            input.auxiliary_inputs,
            input.xi_challenge,
        )
        .map_err(|source| ProvePcsFriOpeningTraceSegmentError::Polynomial {
            unit_index: input.unit_index,
            source: Box::new(source),
        })?;
        opening_values.push(PcsFriOpeningTraceValue {
            unit_index: input.unit_index,
            challenges: input.challenges,
            polynomial,
        });
    }

    build_pcs_fri_opening_segment_from_value_refs(
        schedule,
        query_segment,
        opening_values.iter().map(|value| PcsFriOpeningValueRef {
            unit_index: value.unit_index,
            challenges: value.challenges,
            polynomial: &value.polynomial,
        }),
        opening_values.len(),
    )
    .map_err(|source| ProvePcsFriOpeningTraceSegmentError::Opening {
        source: Box::new(source),
    })
}

pub fn build_pcs_fri_opening_segment_from_trace_segments(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    values: &[ProvePcsFriTranscriptTraceSegmentValues<'_>],
) -> Result<ProofSegment, ProvePcsFriOpeningTraceSegmentError> {
    let transcript_values = build_pcs_fri_transcript_values_from_trace_segments(schedule, values)
        .map_err(|source| {
        ProvePcsFriOpeningTraceSegmentError::TranscriptValues {
            source: Box::new(source),
        }
    })?;
    build_pcs_fri_opening_segment_from_transcript_values(
        schedule,
        query_segment,
        &transcript_values,
    )
    .map_err(|source| ProvePcsFriOpeningTraceSegmentError::Opening {
        source: Box::new(source),
    })
}

fn validate_material_segment_id(
    segment: &ProofSegment,
) -> Result<(), ProvePcsFriTranscriptTraceValuesError> {
    if segment.id == PCS_MATERIAL_MANIFEST_SEGMENT_ID {
        Ok(())
    } else {
        Err(
            ProvePcsFriTranscriptTraceValuesError::InvalidMaterialSegmentId {
                segment_id: segment.id,
            },
        )
    }
}

fn validate_evaluation_segment_id(
    segment: &ProofSegment,
) -> Result<(), ProvePcsFriTranscriptTraceValuesError> {
    if segment.id == PCS_EVALUATION_SEGMENT_ID {
        Ok(())
    } else {
        Err(
            ProvePcsFriTranscriptTraceValuesError::InvalidEvaluationSegmentId {
                segment_id: segment.id,
            },
        )
    }
}

fn root_from_words(words: [u64; 4]) -> Result<[Felt; 4], FieldError> {
    Ok([
        Felt::from_canonical(words[0])?,
        Felt::from_canonical(words[1])?,
        Felt::from_canonical(words[2])?,
        Felt::from_canonical(words[3])?,
    ])
}

fn extension_from_words(words: [u64; 3]) -> Result<Ext3, FieldError> {
    Ok(Ext3::new(
        Felt::from_canonical(words[0])?,
        Felt::from_canonical(words[1])?,
        Felt::from_canonical(words[2])?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lzvm_artifacts::pcs_evaluation_segment::{PcsEvaluationSegment, PcsEvaluationUnitSegment};
    use lzvm_artifacts::pcs_material_segment::{
        PcsMaterialManifestSegment, PcsMaterialManifestUnit,
    };
    use std::cell::Cell;

    #[test]
    fn material_segment_cache_reuses_identical_data() {
        let parses = Cell::new(0);
        let segment = ProofSegment {
            id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
            data: vec![1, 2, 3],
        };
        let mut cache = MaterialSegmentCache::with_parser(|bytes| {
            parses.set(parses.get() + 1);
            assert_eq!(bytes, [1, 2, 3]);
            Ok(PcsMaterialManifestSegment {
                units: vec![material_unit(7)],
            })
        });

        assert_eq!(
            cache.get(&segment).expect("segment should parse").units[0].unit_index,
            7
        );
        assert_eq!(
            cache.get(&segment).expect("segment should be reused").units[0].unit_index,
            7
        );
        assert_eq!(parses.get(), 1);
    }

    #[test]
    fn material_segment_cache_reuses_equal_distinct_data_and_reparses_changed_data() {
        let parses = Cell::new(0);
        let first = ProofSegment {
            id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
            data: vec![1, 2, 3],
        };
        let equal = ProofSegment {
            id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
            data: vec![1, 2, 3],
        };
        let changed = ProofSegment {
            id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
            data: vec![1, 2, 4],
        };
        let corrupt = ProofSegment {
            id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
            data: vec![9],
        };
        let mut cache = MaterialSegmentCache::with_parser(|bytes| {
            parses.set(parses.get() + 1);
            match bytes {
                [1, 2, 3] => Ok(PcsMaterialManifestSegment {
                    units: vec![material_unit(7)],
                }),
                [1, 2, 4] => Ok(PcsMaterialManifestSegment {
                    units: vec![material_unit(8)],
                }),
                [9] => Err(PcsMaterialManifestSegmentError::InvalidMagic),
                _ => panic!("unexpected segment bytes"),
            }
        });

        assert_eq!(
            cache.get(&first).expect("segment should parse").units[0].unit_index,
            7
        );
        assert_eq!(
            cache
                .get(&equal)
                .expect("equal segment should be reused")
                .units[0]
                .unit_index,
            7
        );
        assert_eq!(parses.get(), 1);
        assert_eq!(
            cache
                .get(&changed)
                .expect("changed segment should parse")
                .units[0]
                .unit_index,
            8
        );
        assert_eq!(parses.get(), 2);
        assert!(matches!(
            cache.get(&corrupt),
            Err(ProvePcsFriTranscriptTraceValuesError::MaterialSegment(
                PcsMaterialManifestSegmentError::InvalidMagic
            ))
        ));
        assert_eq!(parses.get(), 3);
    }

    #[test]
    fn evaluation_segment_cache_reuses_identical_data() {
        let parses = Cell::new(0);
        let segment = ProofSegment {
            id: PCS_EVALUATION_SEGMENT_ID,
            data: vec![4, 5, 6],
        };
        let mut cache = EvaluationSegmentCache::with_parser(|bytes| {
            parses.set(parses.get() + 1);
            assert_eq!(bytes, [4, 5, 6]);
            Ok(PcsEvaluationSegment {
                units: vec![evaluation_unit(9)],
            })
        });

        assert_eq!(
            cache.get(&segment).expect("segment should parse").units[0].unit_index,
            9
        );
        assert_eq!(
            cache.get(&segment).expect("segment should be reused").units[0].unit_index,
            9
        );
        assert_eq!(parses.get(), 1);
    }

    #[test]
    fn evaluation_segment_cache_reuses_equal_distinct_data_and_reparses_changed_data() {
        let parses = Cell::new(0);
        let first = ProofSegment {
            id: PCS_EVALUATION_SEGMENT_ID,
            data: vec![4, 5, 6],
        };
        let equal = ProofSegment {
            id: PCS_EVALUATION_SEGMENT_ID,
            data: vec![4, 5, 6],
        };
        let changed = ProofSegment {
            id: PCS_EVALUATION_SEGMENT_ID,
            data: vec![4, 5, 7],
        };
        let corrupt = ProofSegment {
            id: PCS_EVALUATION_SEGMENT_ID,
            data: vec![8],
        };
        let mut cache = EvaluationSegmentCache::with_parser(|bytes| {
            parses.set(parses.get() + 1);
            match bytes {
                [4, 5, 6] => Ok(PcsEvaluationSegment {
                    units: vec![evaluation_unit(9)],
                }),
                [4, 5, 7] => Ok(PcsEvaluationSegment {
                    units: vec![evaluation_unit(10)],
                }),
                [8] => Err(PcsEvaluationSegmentError::InvalidMagic),
                _ => panic!("unexpected segment bytes"),
            }
        });

        assert_eq!(
            cache.get(&first).expect("segment should parse").units[0].unit_index,
            9
        );
        assert_eq!(
            cache
                .get(&equal)
                .expect("equal segment should be reused")
                .units[0]
                .unit_index,
            9
        );
        assert_eq!(parses.get(), 1);
        assert_eq!(
            cache
                .get(&changed)
                .expect("changed segment should parse")
                .units[0]
                .unit_index,
            10
        );
        assert_eq!(parses.get(), 2);
        assert!(matches!(
            cache.get(&corrupt),
            Err(ProvePcsFriTranscriptTraceValuesError::EvaluationSegment(
                PcsEvaluationSegmentError::InvalidMagic
            ))
        ));
        assert_eq!(parses.get(), 3);
    }

    fn material_unit(unit_index: u32) -> PcsMaterialManifestUnit {
        PcsMaterialManifestUnit {
            unit_index,
            plan_digest: [0; 32],
            fixed_column_digest: [0; 32],
            constant_tree_digest: [0; 32],
            constant_tree_root: [0; 4],
            fixed_byte_count: 0,
            constant_tree_byte_count: 0,
            leaf_byte_count: 0,
            node_byte_count: 0,
        }
    }

    fn evaluation_unit(unit_index: u32) -> PcsEvaluationUnitSegment {
        PcsEvaluationUnitSegment {
            unit_index,
            values: vec![[0, 0, 0]],
        }
    }
}

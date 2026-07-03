mod errors;
mod values;

pub use errors::*;
pub use values::*;

use std::collections::{BTreeMap, BTreeSet};

use lzvm_artifacts::pcs_evaluation_segment::{
    parse_pcs_evaluation_segment, PcsEvaluationSegment, PcsEvaluationSegmentError,
    PcsEvaluationUnitSegment, PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::pcs_fri_segment::{
    encode_pcs_fri_opening_segment, PcsFriOpeningSegment, PCS_FRI_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::pcs_material_segment::{
    parse_pcs_material_manifest_segment, PcsMaterialManifestSegment,
    PcsMaterialManifestSegmentError, PcsMaterialManifestUnit, PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::{
    parse_pcs_query_plan_segment, PcsQueryPlanUnit, PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_segment::WitnessCommitmentSegmentIdentity;
use lzvm_field::{Ext3, Felt, FieldError};

use crate::indexing::{index_first_by_key, index_first_position_by_key};
use crate::pcs_fri::{
    build_pcs_fri_opening_unit, build_pcs_fri_opening_unit_from_transcript_commitments_with_timing,
    build_pcs_fri_opening_unit_with_timing, build_pcs_fri_transcript_commitments,
    build_pcs_fri_transcript_commitments_with_timing, PcsFriOpeningBuildRequest,
    PcsFriOpeningBuildTiming, PcsFriTranscriptCommitmentRequest,
};
use crate::pcs_transcript::{derive_pcs_transcript_prefix_challenges, PcsTranscriptPrefixInputs};
#[cfg(not(feature = "cuda"))]
use crate::prove_fri_polynomial::build_pcs_fri_polynomial_values_with_slices_and_fixed_cache;
#[cfg(feature = "cuda")]
use crate::prove_fri_polynomial::build_pcs_fri_polynomial_values_with_slices_stage_sources_and_fixed_cache;
use crate::prove_fri_polynomial::{
    build_pcs_fri_polynomial_values, PcsFriFixedColumnsCache, ProvePcsFriPolynomialTraceInput,
};
use crate::witness_commitment::{
    load_witness_commitment_segment_ref_for_identity, LoadWitnessCommitmentSegmentsError,
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
    units_by_index: BTreeMap<u32, usize>,
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

    fn unit_by_index(
        &mut self,
        segment: &'a ProofSegment,
        unit_index: u32,
    ) -> Result<Option<&PcsMaterialManifestUnit>, ProvePcsFriTranscriptTraceValuesError> {
        let index = self.entry_index(segment)?;
        let entry = &self.entries[index];
        Ok(entry
            .units_by_index
            .get(&unit_index)
            .map(|unit_position| &entry.parsed.units[*unit_position]))
    }

    fn entry_index(
        &mut self,
        segment: &'a ProofSegment,
    ) -> Result<usize, ProvePcsFriTranscriptTraceValuesError> {
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
            return Ok(index);
        }
        let parsed = (self.parser)(&segment.data)
            .map_err(ProvePcsFriTranscriptTraceValuesError::MaterialSegment)?;
        let units_by_index = index_first_position_by_key(&parsed.units, |unit| unit.unit_index);
        self.entries.push(CachedMaterialSegment {
            data: segment.data.as_slice(),
            parsed,
            units_by_index,
        });
        Ok(self.entries.len() - 1)
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
    units_by_identity: BTreeMap<(u32, u32), usize>,
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

    fn unit_by_identity(
        &mut self,
        segment: &'a ProofSegment,
        unit_index: u32,
        trace_instance_index: u32,
    ) -> Result<Option<&PcsEvaluationUnitSegment>, ProvePcsFriTranscriptTraceValuesError> {
        let index = self.entry_index(segment)?;
        let entry = &self.entries[index];
        Ok(entry
            .units_by_identity
            .get(&(unit_index, trace_instance_index))
            .map(|unit_position| &entry.parsed.units[*unit_position]))
    }

    fn entry_index(
        &mut self,
        segment: &'a ProofSegment,
    ) -> Result<usize, ProvePcsFriTranscriptTraceValuesError> {
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
            return Ok(index);
        }
        let parsed = (self.parser)(&segment.data)
            .map_err(ProvePcsFriTranscriptTraceValuesError::EvaluationSegment)?;
        let units_by_identity = index_first_position_by_key(&parsed.units, |unit| {
            (unit.unit_index, unit.trace_instance_index)
        });
        self.entries.push(CachedEvaluationSegment {
            data: segment.data.as_slice(),
            parsed,
            units_by_identity,
        });
        Ok(self.entries.len() - 1)
    }
}

fn same_segment_data(left: &[u8], right: &[u8]) -> bool {
    (left.len() == right.len() && std::ptr::eq(left.as_ptr(), right.as_ptr())) || left == right
}

fn query_plan_units_by_identity(
    units: &[PcsQueryPlanUnit],
) -> BTreeMap<(u32, u32), &PcsQueryPlanUnit> {
    index_first_by_key(units, |unit| (unit.unit_index, unit.trace_instance_index))
}

#[derive(Debug, Clone, Copy)]
struct PcsFriOpeningValueRef<'a> {
    unit_index: usize,
    trace_instance_index: u32,
    challenges: &'a [Ext3],
    polynomial: &'a [Ext3],
}

struct PcsFriOpeningTraceValue<'a> {
    unit_index: usize,
    trace_instance_index: u32,
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
            trace_instance_index: value.trace_instance_index,
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
    build_pcs_fri_opening_segment_from_value_refs_with_timing(
        schedule,
        query_segment,
        values,
        value_count,
        None,
    )
}

fn build_pcs_fri_opening_segment_from_value_refs_with_timing<'a>(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    values: impl IntoIterator<Item = PcsFriOpeningValueRef<'a>>,
    value_count: usize,
    mut timing: Option<&mut PcsFriOpeningBuildTiming>,
) -> Result<ProofSegment, ProvePcsFriOpeningSegmentError> {
    if query_segment.id != PCS_QUERY_PLAN_SEGMENT_ID {
        return Err(ProvePcsFriOpeningSegmentError::InvalidQuerySegmentId {
            segment_id: query_segment.id,
        });
    }
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
    let query_units = query_plan_units_by_identity(&query_plan.units);
    let mut seen_units = BTreeSet::new();
    let mut units = Vec::with_capacity(value_count);
    for input in values {
        if !seen_units.insert((input.unit_index, input.trace_instance_index)) {
            return Err(ProvePcsFriOpeningSegmentError::DuplicateUnitIdentity {
                unit_index: input.unit_index,
                trace_instance_index: input.trace_instance_index,
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
        let query_unit = query_units
            .get(&(unit_index_u32, input.trace_instance_index))
            .copied()
            .ok_or(ProvePcsFriOpeningSegmentError::MissingQueryUnit {
                unit_index: input.unit_index,
            })?;
        let request = PcsFriOpeningBuildRequest {
            unit_index: unit_index_u32,
            trace_instance_index: input.trace_instance_index,
            query_rows: &query_unit.queries,
            challenges: input.challenges,
            polynomial: input.polynomial,
        };
        let opening = match timing.as_deref_mut() {
            Some(timing) => build_pcs_fri_opening_unit_with_timing(unit, request, Some(timing)),
            None => build_pcs_fri_opening_unit(unit, request),
        }
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
            #[cfg(feature = "cuda")]
            stage_source_devices: None,
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
    let mut fixed_columns_cache = PcsFriFixedColumnsCache::default();
    build_pcs_fri_transcript_values_from_trace_refs_with_fixed_cache(
        schedule,
        values,
        &mut fixed_columns_cache,
        None,
    )
}

fn build_pcs_fri_transcript_values_from_trace_refs_with_fixed_cache(
    schedule: &ProveSchedule,
    values: &[ProvePcsFriTranscriptTraceValueRef<'_>],
    fixed_columns_cache: &mut PcsFriFixedColumnsCache,
    mut timing: Option<&mut PcsFriOpeningBuildTiming>,
) -> Result<Vec<ProvePcsFriTranscriptValues>, ProvePcsFriTranscriptTraceValuesError> {
    let mut seen_units = BTreeSet::new();
    let mut out = Vec::with_capacity(values.len());
    for input in values {
        let unit = schedule.units.get(input.unit_index).ok_or(
            ProvePcsFriTranscriptTraceValuesError::UnitIndexOutOfRange {
                unit_index: input.unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        if !seen_units.insert((input.unit_index, 0)) {
            return Err(
                ProvePcsFriTranscriptTraceValuesError::DuplicateUnitIdentity {
                    unit_index: input.unit_index,
                    trace_instance_index: 0,
                },
            );
        }
        let arity = unit.transcript_arity.ok_or(
            ProvePcsFriTranscriptTraceValuesError::MissingTranscriptArity {
                unit_index: input.unit_index,
            },
        )? as usize;
        #[cfg(feature = "cuda")]
        let polynomial = build_pcs_fri_polynomial_values_with_slices_stage_sources_and_fixed_cache(
            ProvePcsFriPolynomialTraceInput {
                unit_index: input.unit_index,
                unit,
                plan_unit: input.execution_unit,
                trace: input.trace,
                publics: input.publics,
                auxiliary_inputs: input.auxiliary_inputs,
                xi_challenge: input.xi_challenge,
                stage_source_devices: input.stage_source_devices,
            },
            fixed_columns_cache,
        )
        .map_err(|source| ProvePcsFriTranscriptTraceValuesError::Polynomial {
            unit_index: input.unit_index,
            source: Box::new(source),
        })?;
        #[cfg(not(feature = "cuda"))]
        let polynomial = build_pcs_fri_polynomial_values_with_slices_and_fixed_cache(
            ProvePcsFriPolynomialTraceInput {
                unit_index: input.unit_index,
                unit,
                plan_unit: input.execution_unit,
                trace: input.trace,
                publics: input.publics,
                auxiliary_inputs: input.auxiliary_inputs,
                xi_challenge: input.xi_challenge,
            },
            fixed_columns_cache,
        )
        .map_err(|source| ProvePcsFriTranscriptTraceValuesError::Polynomial {
            unit_index: input.unit_index,
            source: Box::new(source),
        })?;
        let request = PcsFriTranscriptCommitmentRequest {
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
        };
        let commitments = match timing.as_deref_mut() {
            Some(timing) => {
                build_pcs_fri_transcript_commitments_with_timing(unit, request, Some(timing))
            }
            None => build_pcs_fri_transcript_commitments(unit, request),
        }
        .map_err(|source| ProvePcsFriTranscriptTraceValuesError::Transcript {
            unit_index: input.unit_index,
            source: Box::new(source),
        })?;
        out.push(ProvePcsFriTranscriptValues {
            unit_index: input.unit_index,
            trace_instance_index: 0,
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
            trace_instance_index: input.trace_instance_index,
            execution_unit: input.execution_unit,
            trace: input.trace,
            #[cfg(feature = "cuda")]
            stage_source_devices: None,
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
    build_pcs_fri_transcript_values_from_trace_segment_refs_with_timing(schedule, values, None)
}

pub(crate) fn build_pcs_fri_transcript_values_from_trace_segment_refs_with_timing(
    schedule: &ProveSchedule,
    values: &[ProvePcsFriTranscriptTraceSegmentValueRef<'_>],
    mut timing: Option<&mut PcsFriOpeningBuildTiming>,
) -> Result<Vec<ProvePcsFriTranscriptValues>, ProvePcsFriTranscriptTraceValuesError> {
    let mut out = Vec::with_capacity(values.len());
    let mut seen_units = BTreeSet::new();
    let mut material_cache = MaterialSegmentCache::new();
    let mut evaluation_cache = EvaluationSegmentCache::new();
    let mut fixed_columns_cache = PcsFriFixedColumnsCache::default();
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
        if !seen_units.insert((input.unit_index, input.trace_instance_index)) {
            return Err(
                ProvePcsFriTranscriptTraceValuesError::DuplicateUnitIdentity {
                    unit_index: input.unit_index,
                    trace_instance_index: input.trace_instance_index,
                },
            );
        }
        let arity = unit.transcript_arity.ok_or(
            ProvePcsFriTranscriptTraceValuesError::MissingTranscriptArity {
                unit_index: input.unit_index,
            },
        )? as usize;
        validate_material_segment_id(input.material_segment)?;
        let witness_identity = WitnessCommitmentSegmentIdentity {
            unit_index: unit_index_u32,
            trace_instance_index: input.trace_instance_index,
        };
        validate_evaluation_segment_id(input.evaluation_segment)?;

        let material = material_cache
            .unit_by_index(input.material_segment, unit_index_u32)?
            .ok_or(ProvePcsFriTranscriptTraceValuesError::MissingMaterialUnit {
                unit_index: input.unit_index,
            })?;
        let witness = load_witness_commitment_segment_ref_for_identity(
            &schedule.units,
            input.witness_segment,
            witness_identity,
        )
        .map_err(|source| map_transcript_witness_load_error(input.unit_index, source))?
        .witness;
        let evaluations = evaluation_cache
            .unit_by_identity(
                input.evaluation_segment,
                unit_index_u32,
                input.trace_instance_index,
            )?
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

        let mut built = build_pcs_fri_transcript_values_from_trace_refs_with_fixed_cache(
            schedule,
            &[ProvePcsFriTranscriptTraceValueRef {
                unit_index: input.unit_index,
                execution_unit: input.execution_unit,
                trace: input.trace,
                #[cfg(feature = "cuda")]
                stage_source_devices: input.stage_source_devices,
                publics: input.publics,
                auxiliary_inputs: input.auxiliary_inputs,
                constant_root,
                witness_roots: &witness_roots,
                evaluation_values: &evaluation_values,
                xi_challenge,
                binding_segments: input.binding_segments,
            }],
            &mut fixed_columns_cache,
            timing.as_deref_mut(),
        )?;
        for value in &mut built {
            value.trace_instance_index = input.trace_instance_index;
        }
        out.append(&mut built);
    }
    Ok(out)
}

fn map_transcript_witness_load_error(
    unit_index: usize,
    source: LoadWitnessCommitmentSegmentsError,
) -> ProvePcsFriTranscriptTraceValuesError {
    match source {
        LoadWitnessCommitmentSegmentsError::UnexpectedSegment {
            expected, found, ..
        } => ProvePcsFriTranscriptTraceValuesError::InvalidWitnessSegmentId {
            unit_index,
            expected,
            found,
        },
        LoadWitnessCommitmentSegmentsError::Segment { source, .. } => {
            ProvePcsFriTranscriptTraceValuesError::WitnessSegment { unit_index, source }
        }
        source => ProvePcsFriTranscriptTraceValuesError::WitnessCommitment { unit_index, source },
    }
}

pub fn build_pcs_fri_opening_segment_from_transcript_values(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    values: &[ProvePcsFriTranscriptValues],
) -> Result<ProofSegment, ProvePcsFriOpeningSegmentError> {
    build_pcs_fri_opening_segment_from_transcript_values_with_timing(
        schedule,
        query_segment,
        values,
        None,
    )
}

pub fn build_pcs_fri_opening_segment_from_transcript_values_with_timing(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    values: &[ProvePcsFriTranscriptValues],
    timing: Option<&mut PcsFriOpeningBuildTiming>,
) -> Result<ProofSegment, ProvePcsFriOpeningSegmentError> {
    build_pcs_fri_opening_segment_from_transcript_values_cached_with_timing(
        schedule,
        query_segment,
        values,
        timing,
    )
}

fn build_pcs_fri_opening_segment_from_transcript_values_cached_with_timing(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    values: &[ProvePcsFriTranscriptValues],
    mut timing: Option<&mut PcsFriOpeningBuildTiming>,
) -> Result<ProofSegment, ProvePcsFriOpeningSegmentError> {
    if query_segment.id != PCS_QUERY_PLAN_SEGMENT_ID {
        return Err(ProvePcsFriOpeningSegmentError::InvalidQuerySegmentId {
            segment_id: query_segment.id,
        });
    }
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
    let query_units = query_plan_units_by_identity(&query_plan.units);
    let mut seen_units = BTreeSet::new();
    let mut units = Vec::with_capacity(values.len());
    for input in values {
        if !seen_units.insert((input.unit_index, input.trace_instance_index)) {
            return Err(ProvePcsFriOpeningSegmentError::DuplicateUnitIdentity {
                unit_index: input.unit_index,
                trace_instance_index: input.trace_instance_index,
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
        let query_unit = query_units
            .get(&(unit_index_u32, input.trace_instance_index))
            .copied()
            .ok_or(ProvePcsFriOpeningSegmentError::MissingQueryUnit {
                unit_index: input.unit_index,
            })?;
        let opening = build_pcs_fri_opening_unit_from_transcript_commitments_with_timing(
            unit,
            unit_index_u32,
            input.trace_instance_index,
            &query_unit.queries,
            &input.commitments,
            timing.as_deref_mut(),
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
            trace_instance_index: 0,
            challenges: input.challenges,
            polynomial,
        });
    }

    build_pcs_fri_opening_segment_from_value_refs(
        schedule,
        query_segment,
        opening_values.iter().map(|value| PcsFriOpeningValueRef {
            unit_index: value.unit_index,
            trace_instance_index: value.trace_instance_index,
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
    fn query_plan_unit_index_uses_trace_identity_and_first_match() {
        let units = vec![
            PcsQueryPlanUnit {
                unit_index: 0,
                trace_instance_index: 0,
                queries: vec![11],
            },
            PcsQueryPlanUnit {
                unit_index: 0,
                trace_instance_index: 1,
                queries: vec![22],
            },
            PcsQueryPlanUnit {
                unit_index: 0,
                trace_instance_index: 0,
                queries: vec![33],
            },
        ];

        let indexed = query_plan_units_by_identity(&units);

        assert_eq!(
            indexed
                .get(&(0, 0))
                .expect("unit should exist")
                .queries
                .as_slice(),
            &[11]
        );
        assert_eq!(
            indexed
                .get(&(0, 1))
                .expect("unit should exist")
                .queries
                .as_slice(),
            &[22]
        );
        assert!(indexed.get(&(1, 0)).is_none());
    }

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
            cache
                .unit_by_index(&segment, 7)
                .expect("segment should parse")
                .expect("unit should exist")
                .unit_index,
            7
        );
        assert_eq!(
            cache
                .unit_by_index(&segment, 7)
                .expect("segment should be reused")
                .expect("unit should exist")
                .unit_index,
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
            cache
                .unit_by_index(&first, 7)
                .expect("segment should parse")
                .expect("unit should exist")
                .unit_index,
            7
        );
        assert_eq!(
            cache
                .unit_by_index(&equal, 7)
                .expect("equal segment should be reused")
                .expect("unit should exist")
                .unit_index,
            7
        );
        assert_eq!(parses.get(), 1);
        assert_eq!(
            cache
                .unit_by_index(&changed, 8)
                .expect("changed segment should parse")
                .expect("unit should exist")
                .unit_index,
            8
        );
        assert_eq!(parses.get(), 2);
        assert!(matches!(
            cache.unit_by_index(&corrupt, 9),
            Err(ProvePcsFriTranscriptTraceValuesError::MaterialSegment(
                PcsMaterialManifestSegmentError::InvalidMagic
            ))
        ));
        assert_eq!(parses.get(), 3);
    }

    #[test]
    fn material_segment_cache_indexes_units_by_index() {
        let parses = Cell::new(0);
        let segment = ProofSegment {
            id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
            data: vec![1, 2, 3],
        };
        let mut first = material_unit(7);
        first.constant_tree_root = [71, 0, 0, 0];
        let mut second = material_unit(8);
        second.constant_tree_root = [81, 0, 0, 0];
        let mut duplicate = material_unit(7);
        duplicate.constant_tree_root = [72, 0, 0, 0];
        let mut cache = MaterialSegmentCache::with_parser(|bytes| {
            parses.set(parses.get() + 1);
            assert_eq!(bytes, [1, 2, 3]);
            Ok(PcsMaterialManifestSegment {
                units: vec![first.clone(), second.clone(), duplicate.clone()],
            })
        });

        assert_eq!(
            cache
                .unit_by_index(&segment, 7)
                .expect("segment should parse")
                .expect("unit should exist")
                .constant_tree_root,
            [71, 0, 0, 0]
        );
        assert_eq!(
            cache
                .unit_by_index(&segment, 8)
                .expect("segment should be reused")
                .expect("unit should exist")
                .constant_tree_root,
            [81, 0, 0, 0]
        );
        assert!(cache
            .unit_by_index(&segment, 9)
            .expect("segment should be reused")
            .is_none());
        assert_eq!(parses.get(), 1);
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
            cache
                .unit_by_identity(&segment, 9, 0)
                .expect("segment should parse")
                .expect("unit should exist")
                .unit_index,
            9
        );
        assert_eq!(
            cache
                .unit_by_identity(&segment, 9, 0)
                .expect("segment should be reused")
                .expect("unit should exist")
                .unit_index,
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
            cache
                .unit_by_identity(&first, 9, 0)
                .expect("segment should parse")
                .expect("unit should exist")
                .unit_index,
            9
        );
        assert_eq!(
            cache
                .unit_by_identity(&equal, 9, 0)
                .expect("equal segment should be reused")
                .expect("unit should exist")
                .unit_index,
            9
        );
        assert_eq!(parses.get(), 1);
        assert_eq!(
            cache
                .unit_by_identity(&changed, 10, 0)
                .expect("changed segment should parse")
                .expect("unit should exist")
                .unit_index,
            10
        );
        assert_eq!(parses.get(), 2);
        assert!(matches!(
            cache.unit_by_identity(&corrupt, 8, 0),
            Err(ProvePcsFriTranscriptTraceValuesError::EvaluationSegment(
                PcsEvaluationSegmentError::InvalidMagic
            ))
        ));
        assert_eq!(parses.get(), 3);
    }

    #[test]
    fn evaluation_segment_cache_indexes_units_by_trace_identity() {
        let parses = Cell::new(0);
        let segment = ProofSegment {
            id: PCS_EVALUATION_SEGMENT_ID,
            data: vec![4, 5, 6],
        };
        let mut first = evaluation_unit(9);
        first.values = vec![[91, 0, 0]];
        let mut second = evaluation_unit(9);
        second.trace_instance_index = 1;
        second.values = vec![[92, 0, 0]];
        let mut duplicate = evaluation_unit(9);
        duplicate.values = vec![[93, 0, 0]];
        let mut cache = EvaluationSegmentCache::with_parser(|bytes| {
            parses.set(parses.get() + 1);
            assert_eq!(bytes, [4, 5, 6]);
            Ok(PcsEvaluationSegment {
                units: vec![first.clone(), second.clone(), duplicate.clone()],
            })
        });

        assert_eq!(
            cache
                .unit_by_identity(&segment, 9, 0)
                .expect("segment should parse")
                .expect("unit should exist")
                .values[0],
            [91, 0, 0]
        );
        assert_eq!(
            cache
                .unit_by_identity(&segment, 9, 1)
                .expect("segment should be reused")
                .expect("unit should exist")
                .values[0],
            [92, 0, 0]
        );
        assert!(cache
            .unit_by_identity(&segment, 9, 2)
            .expect("segment should be reused")
            .is_none());
        assert_eq!(parses.get(), 1);
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
            trace_instance_index: 0,
            values: vec![[0, 0, 0]],
        }
    }
}

mod errors;
mod values;

pub use errors::*;
pub use values::*;

use std::collections::BTreeSet;

use lzvm_artifacts::pcs_evaluation_segment::{
    parse_pcs_evaluation_segment, PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::pcs_fri_segment::{
    encode_pcs_fri_opening_segment, PcsFriOpeningSegment, PCS_FRI_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::pcs_material_segment::{
    parse_pcs_material_manifest_segment, PCS_MATERIAL_MANIFEST_SEGMENT_ID,
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
use crate::pcs_transcript::{derive_pcs_transcript_prefix_challenges, PcsTranscriptPrefixInputs};
use crate::prove_fri_polynomial::build_pcs_fri_polynomial_values;
use crate::ProveSchedule;

pub fn build_pcs_fri_opening_segment(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    values: &[ProvePcsFriOpeningValues],
) -> Result<ProofSegment, ProvePcsFriOpeningSegmentError> {
    if query_segment.id != PCS_QUERY_PLAN_SEGMENT_ID {
        return Err(ProvePcsFriOpeningSegmentError::InvalidQuerySegmentId {
            segment_id: query_segment.id,
        });
    }
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
    let mut seen_units = BTreeSet::new();
    let mut units = Vec::with_capacity(values.len());
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
                challenges: &input.challenges,
                polynomial: &input.polynomial,
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
        let polynomial = build_pcs_fri_polynomial_values(
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
                unit_values: &input.auxiliary_inputs.unit_values,
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
    let mut out = Vec::with_capacity(values.len());
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
        if input.material_segment.id != PCS_MATERIAL_MANIFEST_SEGMENT_ID {
            return Err(
                ProvePcsFriTranscriptTraceValuesError::InvalidMaterialSegmentId {
                    segment_id: input.material_segment.id,
                },
            );
        }
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
        if input.evaluation_segment.id != PCS_EVALUATION_SEGMENT_ID {
            return Err(
                ProvePcsFriTranscriptTraceValuesError::InvalidEvaluationSegmentId {
                    segment_id: input.evaluation_segment.id,
                },
            );
        }

        let material = parse_pcs_material_manifest_segment(&input.material_segment.data)
            .map_err(ProvePcsFriTranscriptTraceValuesError::MaterialSegment)?
            .units
            .into_iter()
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
        let evaluations = parse_pcs_evaluation_segment(&input.evaluation_segment.data)
            .map_err(ProvePcsFriTranscriptTraceValuesError::EvaluationSegment)?
            .units
            .into_iter()
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
                unit_values: &input.auxiliary_inputs.unit_values,
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

        let mut built = build_pcs_fri_transcript_values_from_trace(
            schedule,
            &[ProvePcsFriTranscriptTraceValues {
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
    let opening_values = values
        .iter()
        .map(|value| ProvePcsFriOpeningValues {
            unit_index: value.unit_index,
            challenges: value.commitments.challenges.clone(),
            polynomial: value.polynomial.clone(),
        })
        .collect::<Vec<_>>();
    build_pcs_fri_opening_segment(schedule, query_segment, &opening_values)
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
        opening_values.push(ProvePcsFriOpeningValues {
            unit_index: input.unit_index,
            challenges: input.challenges.to_vec(),
            polynomial,
        });
    }

    build_pcs_fri_opening_segment(schedule, query_segment, &opening_values).map_err(|source| {
        ProvePcsFriOpeningTraceSegmentError::Opening {
            source: Box::new(source),
        }
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

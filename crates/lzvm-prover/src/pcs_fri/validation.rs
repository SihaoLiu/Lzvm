use lzvm_artifacts::pcs_fri_segment::{PcsFriOpeningUnitSegment, PCS_FRI_OPENING_SEGMENT_ID};
use lzvm_artifacts::pcs_query_segment::PcsQueryPlanUnit;
use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{Ext3, Felt};

use super::errors::{
    LoadPcsFriOpeningSegmentError, ValidateOptionalPcsFriOpeningProofSegmentsError,
    ValidatePcsFriOpeningFoldUnitsError, ValidatePcsFriOpeningSegmentsError,
};
use super::requests::{
    PcsFriOpeningFoldRequest, ValidateOptionalPcsFriOpeningProofSegmentsRequest,
};
use super::{
    load_pcs_fri_opening_segment_from_segments, verify_fri_last_level_root,
    verify_fri_opening_folds, verify_fri_query_path,
};
use crate::pcs_query_plan::{
    load_pcs_query_plan_from_segments, uses_transcript_pcs_query_plan_inputs,
};
use crate::pcs_transcript_segments::{
    derive_pcs_transcript_unit_challenges_from_proof_segments, PcsTranscriptUnitChallenges,
};
use crate::verifier_query::{
    validate_verifier_query_outputs_from_segments, VerifierFriQueryOutputSegmentsRequest,
};
use crate::ProveUnitSchedule;

pub fn validate_pcs_fri_opening_segments(
    units: &[ProveUnitSchedule],
    segments: &[ProofSegment],
) -> Result<(), ValidatePcsFriOpeningSegmentsError> {
    let query_plan = load_pcs_query_plan_from_segments(segments)
        .map_err(ValidatePcsFriOpeningSegmentsError::QueryPlan)?;
    let opening = load_pcs_fri_opening_segment_from_segments(segments)
        .map_err(ValidatePcsFriOpeningSegmentsError::Opening)?;
    validate_pcs_fri_opening_units(units, &query_plan.units, &opening.units)
}

fn validate_pcs_fri_opening_units(
    units: &[ProveUnitSchedule],
    query_units: &[PcsQueryPlanUnit],
    opening_units: &[PcsFriOpeningUnitSegment],
) -> Result<(), ValidatePcsFriOpeningSegmentsError> {
    if opening_units.len() != query_units.len() {
        return Err(ValidatePcsFriOpeningSegmentsError::UnitCountMismatch);
    }

    for query_unit in query_units {
        let unit_index = usize::try_from(query_unit.unit_index)
            .map_err(|_| ValidatePcsFriOpeningSegmentsError::UnitIndexOverflow)?;
        let unit = units
            .get(unit_index)
            .ok_or(ValidatePcsFriOpeningSegmentsError::UnitMismatch { unit_index })?;
        let opening_unit = opening_units
            .iter()
            .find(|unit| {
                unit.unit_index == query_unit.unit_index
                    && unit.trace_instance_index == query_unit.trace_instance_index
            })
            .ok_or(ValidatePcsFriOpeningSegmentsError::UnitMismatch { unit_index })?;
        let final_len = checked_power_of_two_validation(unit.final_layer_bits)
            .ok_or(ValidatePcsFriOpeningSegmentsError::FinalLayerSizeOverflow)?;
        if opening_unit.final_polynomial.len() != final_len
            || opening_unit.layers.len() != unit.fri_layers.len()
        {
            return Err(ValidatePcsFriOpeningSegmentsError::UnitMismatch { unit_index });
        }
        for value in &opening_unit.final_polynomial {
            field_extension_from_words(*value)?;
        }

        for (layer_offset, (layer, expected_layer)) in opening_unit
            .layers
            .iter()
            .zip(unit.fri_layers.iter())
            .enumerate()
        {
            let expected_layer_index = u32::try_from(layer_offset)
                .map_err(|_| ValidatePcsFriOpeningSegmentsError::UnitMismatch { unit_index })?;
            let arity = usize::try_from(unit.merkle_tree_arity)
                .map_err(|_| ValidatePcsFriOpeningSegmentsError::ArityOverflow)?;
            let last_level_count = expected_last_level_digest_count(
                expected_layer.output_bits,
                arity,
                unit.last_level_verification,
            )?;
            if layer.layer_index != expected_layer_index
                || layer.queries.len() != query_unit.queries.len()
                || layer.last_level.len() != last_level_count
            {
                return Err(ValidatePcsFriOpeningSegmentsError::UnitMismatch { unit_index });
            }
            let root = field_digest_from_words(layer.root)?;
            let last_level = layer
                .last_level
                .iter()
                .map(|digest| field_digest_from_words(*digest))
                .collect::<Result<Vec<_>, _>>()?;
            if !last_level.is_empty() {
                let valid =
                    verify_fri_last_level_root(root, arity, &last_level).map_err(|source| {
                        ValidatePcsFriOpeningSegmentsError::Merkle { unit_index, source }
                    })?;
                if !valid {
                    return Err(ValidatePcsFriOpeningSegmentsError::UnitMismatch { unit_index });
                }
            }

            let output_domain = checked_power_of_two_validation(expected_layer.output_bits)
                .ok_or(ValidatePcsFriOpeningSegmentsError::LayerSizeOverflow)?;
            let output_domain_u64 = u64::try_from(output_domain)
                .map_err(|_| ValidatePcsFriOpeningSegmentsError::LayerSizeOverflow)?;
            let expected_value_count = usize::try_from(expected_layer.folding_factor)
                .map_err(|_| ValidatePcsFriOpeningSegmentsError::FoldingWidthOverflow)?;
            let expected_sibling_levels = expected_fri_sibling_level_count(
                expected_layer.output_bits,
                arity,
                unit.last_level_verification,
            )?;
            for (query, source_row) in layer.queries.iter().zip(query_unit.queries.iter()) {
                if query.row_index != source_row % output_domain_u64
                    || query.values.len() != expected_value_count
                    || query.siblings.len() != expected_sibling_levels
                {
                    return Err(ValidatePcsFriOpeningSegmentsError::UnitMismatch { unit_index });
                }
                let values = query
                    .values
                    .iter()
                    .map(|value| field_extension_from_words(*value))
                    .collect::<Result<Vec<_>, _>>()?;
                let siblings = query
                    .siblings
                    .iter()
                    .map(|sibling_level| {
                        if sibling_level.siblings.len() + 1 != arity {
                            return Err(ValidatePcsFriOpeningSegmentsError::UnitMismatch {
                                unit_index,
                            });
                        }
                        sibling_level
                            .siblings
                            .iter()
                            .map(|digest| field_digest_from_words(*digest))
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let valid = verify_fri_query_path(
                    root,
                    &last_level,
                    arity,
                    query.row_index,
                    &values,
                    &siblings,
                )
                .map_err(|source| ValidatePcsFriOpeningSegmentsError::Merkle {
                    unit_index,
                    source,
                })?;
                if !valid {
                    return Err(ValidatePcsFriOpeningSegmentsError::UnitMismatch { unit_index });
                }
            }
        }
    }
    Ok(())
}

pub fn validate_pcs_fri_opening_folds_from_units(
    units: &[ProveUnitSchedule],
    query_units: &[PcsQueryPlanUnit],
    opening_units: &[PcsFriOpeningUnitSegment],
    transcript_challenges: &[PcsTranscriptUnitChallenges],
) -> Result<(), ValidatePcsFriOpeningFoldUnitsError> {
    for query_unit in query_units {
        let unit_index = usize::try_from(query_unit.unit_index)
            .map_err(|_| ValidatePcsFriOpeningFoldUnitsError::UnitIndexOverflow)?;
        let unit = units
            .get(unit_index)
            .ok_or(ValidatePcsFriOpeningFoldUnitsError::UnitMismatch { unit_index })?;
        let opening_unit = opening_units
            .iter()
            .find(|unit| {
                unit.unit_index == query_unit.unit_index
                    && unit.trace_instance_index == query_unit.trace_instance_index
            })
            .ok_or(ValidatePcsFriOpeningFoldUnitsError::UnitMismatch { unit_index })?;
        let challenges = transcript_challenges
            .iter()
            .find(|unit| {
                unit.unit_index == query_unit.unit_index
                    && unit.trace_instance_index == query_unit.trace_instance_index
            })
            .ok_or(ValidatePcsFriOpeningFoldUnitsError::UnitMismatch { unit_index })?;
        let valid = verify_fri_opening_folds(
            unit,
            PcsFriOpeningFoldRequest {
                unit_index: query_unit.unit_index,
                query_rows: &query_unit.queries,
                challenges: &challenges.challenges,
                fri: opening_unit,
            },
        )
        .map_err(|source| ValidatePcsFriOpeningFoldUnitsError::Fold { unit_index, source })?;
        if !valid {
            return Err(ValidatePcsFriOpeningFoldUnitsError::UnitMismatch { unit_index });
        }
    }
    Ok(())
}

pub fn validate_optional_pcs_fri_opening_proof_segments(
    request: ValidateOptionalPcsFriOpeningProofSegmentsRequest<'_>,
) -> Result<(), ValidateOptionalPcsFriOpeningProofSegmentsError> {
    validate_optional_pcs_fri_opening_proof_segments_inner(request, None)
}

pub(crate) fn validate_optional_pcs_fri_opening_proof_segments_with_transcript_challenges(
    request: ValidateOptionalPcsFriOpeningProofSegmentsRequest<'_>,
    transcript_challenges: &[PcsTranscriptUnitChallenges],
) -> Result<(), ValidateOptionalPcsFriOpeningProofSegmentsError> {
    validate_optional_pcs_fri_opening_proof_segments_inner(request, Some(transcript_challenges))
}

fn validate_optional_pcs_fri_opening_proof_segments_inner(
    request: ValidateOptionalPcsFriOpeningProofSegmentsRequest<'_>,
    precomputed_transcript_challenges: Option<&[PcsTranscriptUnitChallenges]>,
) -> Result<(), ValidateOptionalPcsFriOpeningProofSegmentsError> {
    if request.fri_opening_required_units.len() != request.schedule.units.len() {
        return Err(
            ValidateOptionalPcsFriOpeningProofSegmentsError::RequiredUnitCountMismatch {
                expected: request.schedule.units.len(),
                found: request.fri_opening_required_units.len(),
            },
        );
    }

    if !request
        .segments
        .iter()
        .any(|segment| segment.id == PCS_FRI_OPENING_SEGMENT_ID)
    {
        if uses_transcript_pcs_query_plan_inputs(request.segments) {
            return Err(
                ValidateOptionalPcsFriOpeningProofSegmentsError::OpeningSegment(
                    LoadPcsFriOpeningSegmentError::MissingSegment,
                ),
            );
        }
        if seeded_query_plan_requires_fri_opening(request)? {
            return Err(
                ValidateOptionalPcsFriOpeningProofSegmentsError::OpeningSegment(
                    LoadPcsFriOpeningSegmentError::MissingSegment,
                ),
            );
        }
        return Ok(());
    }

    if !uses_transcript_pcs_query_plan_inputs(request.segments) {
        return Err(ValidateOptionalPcsFriOpeningProofSegmentsError::UnboundOpeningSegment);
    }

    let query_plan = load_pcs_query_plan_from_segments(request.segments).map_err(|source| {
        ValidateOptionalPcsFriOpeningProofSegmentsError::Opening(
            ValidatePcsFriOpeningSegmentsError::QueryPlan(source),
        )
    })?;
    let opening =
        load_pcs_fri_opening_segment_from_segments(request.segments).map_err(|source| {
            ValidateOptionalPcsFriOpeningProofSegmentsError::Opening(
                ValidatePcsFriOpeningSegmentsError::Opening(source),
            )
        })?;
    validate_pcs_fri_opening_units(&request.schedule.units, &query_plan.units, &opening.units)
        .map_err(ValidateOptionalPcsFriOpeningProofSegmentsError::Opening)?;

    let owned_transcript_challenges;
    let transcript_challenges = match precomputed_transcript_challenges {
        Some(transcript_challenges) => transcript_challenges,
        None => {
            owned_transcript_challenges =
                derive_pcs_transcript_unit_challenges_from_proof_segments(
                    request.schedule,
                    request.public_values,
                    request.segments,
                )
                .map_err(ValidateOptionalPcsFriOpeningProofSegmentsError::Transcript)?;
            &owned_transcript_challenges
        }
    };
    validate_pcs_fri_opening_folds_from_units(
        &request.schedule.units,
        &query_plan.units,
        &opening.units,
        &transcript_challenges,
    )
    .map_err(ValidateOptionalPcsFriOpeningProofSegmentsError::Fold)?;
    validate_verifier_query_outputs_from_segments(VerifierFriQueryOutputSegmentsRequest {
        units: &request.schedule.units,
        verifier_codes: request.verifier_codes,
        global_info: request.global_info,
        public_values: request.public_values,
        query_units: &query_plan.units,
        opening_units: &opening.units,
        transcript_challenges: &transcript_challenges,
        segments: request.segments,
    })
    .map_err(ValidateOptionalPcsFriOpeningProofSegmentsError::VerifierQuery)
}

fn seeded_query_plan_requires_fri_opening(
    request: ValidateOptionalPcsFriOpeningProofSegmentsRequest<'_>,
) -> Result<bool, ValidateOptionalPcsFriOpeningProofSegmentsError> {
    if !request
        .fri_opening_required_units
        .iter()
        .any(|required| *required)
    {
        return Ok(false);
    }

    let query_plan = load_pcs_query_plan_from_segments(request.segments)
        .map_err(ValidateOptionalPcsFriOpeningProofSegmentsError::QueryPlan)?;
    for query_unit in query_plan.units {
        let unit_index = usize::try_from(query_unit.unit_index)
            .map_err(|_| ValidateOptionalPcsFriOpeningProofSegmentsError::UnitIndexOverflow)?;
        if request
            .fri_opening_required_units
            .get(unit_index)
            .copied()
            .unwrap_or(false)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn expected_fri_sibling_level_count(
    output_bits: u32,
    arity: usize,
    last_level_verification: u32,
) -> Result<usize, ValidatePcsFriOpeningSegmentsError> {
    Ok(expected_fri_tree_shape(output_bits, arity, last_level_verification)?.0)
}

fn expected_last_level_digest_count(
    output_bits: u32,
    arity: usize,
    last_level_verification: u32,
) -> Result<usize, ValidatePcsFriOpeningSegmentsError> {
    Ok(expected_fri_tree_shape(output_bits, arity, last_level_verification)?.1)
}

fn expected_fri_tree_shape(
    output_bits: u32,
    arity: usize,
    last_level_verification: u32,
) -> Result<(usize, usize), ValidatePcsFriOpeningSegmentsError> {
    if arity < 2 || !arity.is_power_of_two() {
        return Err(ValidatePcsFriOpeningSegmentsError::InvalidTreeShape);
    }
    let mut count = checked_power_of_two_validation(output_bits)
        .ok_or(ValidatePcsFriOpeningSegmentsError::LayerSizeOverflow)?;
    let target = if last_level_verification == 0 {
        1
    } else {
        checked_pow_validation(arity, last_level_verification)
            .ok_or(ValidatePcsFriOpeningSegmentsError::LastLevelCountOverflow)?
    };
    let mut sibling_levels = 0_usize;
    while count > target {
        count = count.div_ceil(arity);
        sibling_levels = sibling_levels
            .checked_add(1)
            .ok_or(ValidatePcsFriOpeningSegmentsError::LevelCountOverflow)?;
    }
    if last_level_verification == 0 {
        Ok((sibling_levels, 0))
    } else {
        Ok((sibling_levels, count))
    }
}

fn checked_power_of_two_validation(bits: u32) -> Option<usize> {
    1_usize.checked_shl(bits)
}

fn checked_pow_validation(base: usize, power: u32) -> Option<usize> {
    let mut out = 1_usize;
    for _ in 0..power {
        out = out.checked_mul(base)?;
    }
    Some(out)
}

fn field_extension_from_words(words: [u64; 3]) -> Result<Ext3, ValidatePcsFriOpeningSegmentsError> {
    Ok(Ext3::new(
        Felt::from_canonical(words[0]).map_err(ValidatePcsFriOpeningSegmentsError::FieldValue)?,
        Felt::from_canonical(words[1]).map_err(ValidatePcsFriOpeningSegmentsError::FieldValue)?,
        Felt::from_canonical(words[2]).map_err(ValidatePcsFriOpeningSegmentsError::FieldValue)?,
    ))
}

fn field_digest_from_words(
    words: [u64; 4],
) -> Result<[Felt; 4], ValidatePcsFriOpeningSegmentsError> {
    let mut out = [Felt::ZERO; 4];
    for (target, value) in out.iter_mut().zip(words) {
        *target =
            Felt::from_canonical(value).map_err(ValidatePcsFriOpeningSegmentsError::FieldDigest)?;
    }
    Ok(out)
}

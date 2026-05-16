mod fold;
mod merkle;
mod types;

pub use fold::{verify_fri_fold, PcsFriFoldError};
pub use merkle::{verify_fri_last_level_root, verify_fri_query_path, PcsFriMerkleError};
pub use types::*;

use crate::pcs_query_plan::{
    load_pcs_query_plan_from_segments, uses_transcript_pcs_query_plan_inputs,
};
use crate::pcs_transcript::{absorb_binding_segments, absorb_commit_values, PcsTranscriptError};
use crate::pcs_transcript_segments::{
    derive_pcs_transcript_unit_challenges_from_proof_segments, PcsTranscriptUnitChallenges,
};
use crate::verifier_query::{
    validate_verifier_query_outputs_from_segments, VerifierFriQueryOutputSegmentsRequest,
};
use crate::ProveUnitSchedule;
use lzvm_artifacts::pcs_fri_segment::{
    parse_pcs_fri_opening_segment, PcsFriOpeningLayerSegment, PcsFriOpeningQuerySegment,
    PcsFriOpeningSegment, PcsFriOpeningUnitSegment, PCS_FRI_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::PcsQueryPlanUnit;
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::setup_info::StageValue;
use lzvm_field::{Ext3, Felt, FieldError, PoseidonTranscript};

pub fn load_pcs_fri_opening_segment_from_segments(
    segments: &[ProofSegment],
) -> Result<PcsFriOpeningSegment, LoadPcsFriOpeningSegmentError> {
    let segment = segments
        .iter()
        .find(|segment| segment.id == PCS_FRI_OPENING_SEGMENT_ID)
        .ok_or(LoadPcsFriOpeningSegmentError::MissingSegment)?;
    parse_pcs_fri_opening_segment(&segment.data).map_err(LoadPcsFriOpeningSegmentError::Segment)
}

pub fn load_pcs_fri_opening_unit_from_segments(
    unit_index: usize,
    segments: &[ProofSegment],
) -> Result<PcsFriOpeningUnitSegment, LoadPcsFriOpeningUnitError> {
    let opening = load_pcs_fri_opening_segment_from_segments(segments)?;
    let unit_index_u32 =
        u32::try_from(unit_index).map_err(|_| LoadPcsFriOpeningUnitError::UnitIndexOverflow)?;
    opening
        .units
        .into_iter()
        .find(|unit| unit.unit_index == unit_index_u32)
        .ok_or(LoadPcsFriOpeningUnitError::MissingUnit { unit_index })
}

pub fn validate_pcs_fri_opening_segments(
    units: &[ProveUnitSchedule],
    segments: &[ProofSegment],
) -> Result<(), ValidatePcsFriOpeningSegmentsError> {
    let query_plan = load_pcs_query_plan_from_segments(segments)
        .map_err(ValidatePcsFriOpeningSegmentsError::QueryPlan)?;
    let opening = load_pcs_fri_opening_segment_from_segments(segments)
        .map_err(ValidatePcsFriOpeningSegmentsError::Opening)?;
    if opening.units.len() != query_plan.units.len() {
        return Err(ValidatePcsFriOpeningSegmentsError::UnitCountMismatch);
    }

    for query_unit in &query_plan.units {
        let unit_index = usize::try_from(query_unit.unit_index)
            .map_err(|_| ValidatePcsFriOpeningSegmentsError::UnitIndexOverflow)?;
        let unit = units
            .get(unit_index)
            .ok_or(ValidatePcsFriOpeningSegmentsError::UnitMismatch { unit_index })?;
        let opening_unit = opening
            .units
            .iter()
            .find(|unit| unit.unit_index == query_unit.unit_index)
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
            .find(|unit| unit.unit_index == query_unit.unit_index)
            .ok_or(ValidatePcsFriOpeningFoldUnitsError::UnitMismatch { unit_index })?;
        let challenges = transcript_challenges
            .iter()
            .find(|unit| unit.unit_index == query_unit.unit_index)
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
    if !request
        .segments
        .iter()
        .any(|segment| segment.id == PCS_FRI_OPENING_SEGMENT_ID)
    {
        return Ok(());
    }

    validate_pcs_fri_opening_segments(&request.schedule.units, request.segments)
        .map_err(ValidateOptionalPcsFriOpeningProofSegmentsError::Opening)?;
    if !uses_transcript_pcs_query_plan_inputs(request.segments) {
        return Ok(());
    }

    let query_plan = load_pcs_query_plan_from_segments(request.segments)
        .map_err(ValidateOptionalPcsFriOpeningProofSegmentsError::QueryPlan)?;
    let opening = load_pcs_fri_opening_segment_from_segments(request.segments)
        .map_err(ValidateOptionalPcsFriOpeningProofSegmentsError::OpeningSegment)?;
    let transcript_challenges = derive_pcs_transcript_unit_challenges_from_proof_segments(
        request.schedule,
        request.public_values,
        request.segments,
    )
    .map_err(ValidateOptionalPcsFriOpeningProofSegmentsError::Transcript)?;
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

pub fn build_pcs_fri_transcript_commitments(
    schedule: &ProveUnitSchedule,
    request: PcsFriTranscriptCommitmentRequest<'_>,
) -> Result<PcsFriTranscriptCommitments, PcsFriTranscriptCommitmentError> {
    if schedule.fri_layers.is_empty() {
        return Err(PcsFriOpeningBuildError::EmptyFriLayers.into());
    }

    let (mut transcript, mut challenges) = build_fri_transcript_prefix(request)?;
    challenges.push(Ext3::ZERO);

    let arity = usize::try_from(schedule.merkle_tree_arity)
        .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
    let mut current = request.polynomial.to_vec();
    let mut current_bits = schedule.fri_layers[0].input_bits;
    let expected_initial_len = build_domain_size(current_bits)?;
    if current.len() != expected_initial_len {
        return Err(PcsFriOpeningBuildError::PolynomialLengthMismatch {
            layer_index: 0,
            expected: expected_initial_len,
            found: current.len(),
        }
        .into());
    }

    let mut layer_roots = Vec::with_capacity(schedule.fri_layers.len());
    for (layer_index, layer) in schedule.fri_layers.iter().enumerate() {
        if layer.input_bits != current_bits {
            return Err(PcsFriOpeningBuildError::LayerInputMismatch {
                layer_index,
                expected: current_bits,
                found: layer.input_bits,
            }
            .into());
        }
        if layer.output_bits >= layer.input_bits {
            return Err(PcsFriOpeningBuildError::InvalidLayerBits {
                layer_index,
                input_bits: layer.input_bits,
                output_bits: layer.output_bits,
            }
            .into());
        }

        let output_size = build_domain_size(layer.output_bits)?;
        let folding_factor = usize::try_from(layer.folding_factor)
            .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
        let expected_folding_factor = build_domain_size(layer.input_bits - layer.output_bits)?;
        if folding_factor != expected_folding_factor {
            return Err(PcsFriOpeningBuildError::FoldingFactorMismatch {
                layer_index,
                expected: expected_folding_factor,
                found: folding_factor,
            }
            .into());
        }

        let grouped_values =
            group_fri_layer_values(layer_index, &current, output_size, folding_factor)?;
        let tree =
            merkle::build_fri_layer_tree(&grouped_values, arity, schedule.last_level_verification)?;
        layer_roots.push(tree.root);
        transcript.put(&tree.root);
        let challenge = transcript.get_field();
        challenges.push(challenge);

        let mut next = Vec::with_capacity(output_size);
        for (row_index, values) in grouped_values.iter().enumerate() {
            next.push(
                verify_fri_fold(
                    schedule.extended_domain_bits,
                    layer.output_bits,
                    layer.input_bits,
                    challenge,
                    u64::try_from(row_index)
                        .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?,
                    values,
                )
                .map_err(PcsFriOpeningBuildError::from)?,
            );
        }
        current = next;
        current_bits = layer.output_bits;
    }

    if current_bits != schedule.final_layer_bits {
        return Err(PcsFriOpeningBuildError::FinalLayerMismatch {
            expected: schedule.final_layer_bits,
            found: current_bits,
        }
        .into());
    }

    let final_values = flatten_extension_values_for_transcript(&current);
    absorb_commit_values(
        &mut transcript,
        request.arity,
        request.hash_values,
        &final_values,
    )?;
    let final_query_challenge = transcript.get_field();
    challenges.push(final_query_challenge);

    Ok(PcsFriTranscriptCommitments {
        challenges,
        layer_roots,
        final_polynomial: current,
        final_query_challenge,
    })
}

pub fn build_pcs_fri_opening_unit(
    schedule: &ProveUnitSchedule,
    request: PcsFriOpeningBuildRequest<'_>,
) -> Result<PcsFriOpeningUnitSegment, PcsFriOpeningBuildError> {
    if schedule.fri_layers.is_empty() {
        return Err(PcsFriOpeningBuildError::EmptyFriLayers);
    }

    let query_count = usize::try_from(schedule.query_count)
        .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
    if request.query_rows.len() != query_count {
        return Err(PcsFriOpeningBuildError::QueryRowCountMismatch {
            expected: query_count,
            found: request.query_rows.len(),
        });
    }

    let arity = usize::try_from(schedule.merkle_tree_arity)
        .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
    let mut current = request.polynomial.to_vec();
    let mut current_bits = schedule.fri_layers[0].input_bits;
    let expected_initial_len = build_domain_size(current_bits)?;
    if current.len() != expected_initial_len {
        return Err(PcsFriOpeningBuildError::PolynomialLengthMismatch {
            layer_index: 0,
            expected: expected_initial_len,
            found: current.len(),
        });
    }

    let mut layers = Vec::with_capacity(schedule.fri_layers.len());
    for (layer_index, layer) in schedule.fri_layers.iter().enumerate() {
        if layer.input_bits != current_bits {
            return Err(PcsFriOpeningBuildError::LayerInputMismatch {
                layer_index,
                expected: current_bits,
                found: layer.input_bits,
            });
        }
        if layer.output_bits >= layer.input_bits {
            return Err(PcsFriOpeningBuildError::InvalidLayerBits {
                layer_index,
                input_bits: layer.input_bits,
                output_bits: layer.output_bits,
            });
        }

        let output_size = build_domain_size(layer.output_bits)?;
        let folding_factor = usize::try_from(layer.folding_factor)
            .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
        let expected_folding_factor = build_domain_size(layer.input_bits - layer.output_bits)?;
        if folding_factor != expected_folding_factor {
            return Err(PcsFriOpeningBuildError::FoldingFactorMismatch {
                layer_index,
                expected: expected_folding_factor,
                found: folding_factor,
            });
        }

        let grouped_values =
            group_fri_layer_values(layer_index, &current, output_size, folding_factor)?;
        let tree =
            merkle::build_fri_layer_tree(&grouped_values, arity, schedule.last_level_verification)?;
        let layer_index_u32 =
            u32::try_from(layer_index).map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
        let output_size_u64 =
            u64::try_from(output_size).map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
        let queries = request
            .query_rows
            .iter()
            .map(|query_row| {
                let row_index = *query_row % output_size_u64;
                let row_index_usize = usize::try_from(row_index)
                    .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
                let values = grouped_values[row_index_usize]
                    .iter()
                    .map(|value| value.to_u64s())
                    .collect();
                let siblings = tree.query_siblings(row_index_usize)?;
                Ok(PcsFriOpeningQuerySegment {
                    row_index,
                    values,
                    siblings,
                })
            })
            .collect::<Result<Vec<_>, PcsFriOpeningBuildError>>()?;

        layers.push(PcsFriOpeningLayerSegment {
            layer_index: layer_index_u32,
            root: merkle::digest_to_u64s(tree.root),
            last_level: tree
                .last_level
                .iter()
                .copied()
                .map(merkle::digest_to_u64s)
                .collect(),
            queries,
        });

        let layer_challenge_start = request
            .challenges
            .len()
            .checked_sub(schedule.fri_layers.len())
            .and_then(|index| index.checked_sub(1))
            .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
        let challenge_index = layer_challenge_start
            .checked_add(layer_index)
            .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
        let challenge = *request.challenges.get(challenge_index).ok_or(
            PcsFriOpeningBuildError::MissingChallenge {
                index: challenge_index,
                len: request.challenges.len(),
            },
        )?;
        let mut next = Vec::with_capacity(output_size);
        for (row_index, values) in grouped_values.iter().enumerate() {
            next.push(verify_fri_fold(
                schedule.extended_domain_bits,
                layer.output_bits,
                layer.input_bits,
                challenge,
                u64::try_from(row_index).map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?,
                values,
            )?);
        }
        current = next;
        current_bits = layer.output_bits;
    }

    if current_bits != schedule.final_layer_bits {
        return Err(PcsFriOpeningBuildError::FinalLayerMismatch {
            expected: schedule.final_layer_bits,
            found: current_bits,
        });
    }

    Ok(PcsFriOpeningUnitSegment {
        unit_index: request.unit_index,
        layers,
        final_polynomial: current.iter().map(|value| value.to_u64s()).collect(),
    })
}

pub fn verify_fri_opening_folds(
    schedule: &ProveUnitSchedule,
    request: PcsFriOpeningFoldRequest<'_>,
) -> Result<bool, PcsFriOpeningFoldError> {
    if request.unit_index != request.fri.unit_index {
        return Err(PcsFriOpeningFoldError::UnitIndexMismatch {
            expected: request.unit_index,
            found: request.fri.unit_index,
        });
    }

    let query_count = usize::try_from(schedule.query_count)
        .map_err(|_| PcsFriOpeningFoldError::LengthOverflow)?;
    if request.query_rows.len() != query_count {
        return Err(PcsFriOpeningFoldError::QueryRowCountMismatch {
            expected: query_count,
            found: request.query_rows.len(),
        });
    }
    if request.fri.layers.len() != schedule.fri_layers.len() {
        return Err(PcsFriOpeningFoldError::LayerCountMismatch {
            expected: schedule.fri_layers.len(),
            found: request.fri.layers.len(),
        });
    }

    let layers = ordered_opening_layers(request.fri, schedule.fri_layers.len())?;
    for layer in &layers {
        if layer.queries.len() != query_count {
            return Err(PcsFriOpeningFoldError::LayerQueryCountMismatch {
                layer_index: layer.layer_index,
                expected: query_count,
                found: layer.queries.len(),
            });
        }
    }

    for (query_index, query_row) in request.query_rows.iter().enumerate() {
        for (layer_index, (layer_plan, opening_layer)) in
            schedule.fri_layers.iter().zip(layers.iter()).enumerate()
        {
            let output_size = domain_size(layer_plan.output_bits)?;
            let expected_row = query_row % output_size;
            let query = &opening_layer.queries[query_index];
            if query.row_index != expected_row {
                return Err(PcsFriOpeningFoldError::LayerQueryRowMismatch {
                    layer_index: opening_layer.layer_index,
                    query_index,
                    expected: expected_row,
                    found: query.row_index,
                });
            }

            let values = query
                .values
                .iter()
                .map(|value| convert_ext(*value))
                .collect::<Result<Vec<_>, PcsFriOpeningFoldError>>()?;
            let layer_challenge_start = request
                .challenges
                .len()
                .checked_sub(schedule.fri_layers.len())
                .and_then(|index| index.checked_sub(1))
                .ok_or(PcsFriOpeningFoldError::LengthOverflow)?;
            let challenge_index = layer_challenge_start
                .checked_add(layer_index)
                .ok_or(PcsFriOpeningFoldError::LengthOverflow)?;
            let challenge = *request.challenges.get(challenge_index).ok_or(
                PcsFriOpeningFoldError::MissingChallenge {
                    index: challenge_index,
                    len: request.challenges.len(),
                },
            )?;
            let folded = verify_fri_fold(
                schedule.extended_domain_bits,
                layer_plan.output_bits,
                layer_plan.input_bits,
                challenge,
                expected_row,
                &values,
            )?;

            let target = if let Some(next_plan) = schedule.fri_layers.get(layer_index + 1) {
                let next_output_size = domain_size(next_plan.output_bits)?;
                let value_index = usize::try_from(expected_row / next_output_size)
                    .map_err(|_| PcsFriOpeningFoldError::LengthOverflow)?;
                let next_layer = layers[layer_index + 1];
                let next_query = &next_layer.queries[query_index];
                let value = next_query.values.get(value_index).ok_or(
                    PcsFriOpeningFoldError::LayerValueIndexOutOfRange {
                        layer_index: next_layer.layer_index,
                        query_index,
                        value_index,
                        len: next_query.values.len(),
                    },
                )?;
                convert_ext(*value)?
            } else {
                let final_index = usize::try_from(expected_row)
                    .map_err(|_| PcsFriOpeningFoldError::LengthOverflow)?;
                let value = request.fri.final_polynomial.get(final_index).ok_or(
                    PcsFriOpeningFoldError::FinalIndexOutOfRange {
                        query_index,
                        index: final_index,
                        len: request.fri.final_polynomial.len(),
                    },
                )?;
                convert_ext(*value)?
            };

            if folded != target {
                return Ok(false);
            }
        }
    }

    Ok(true)
}

fn build_fri_transcript_prefix(
    request: PcsFriTranscriptCommitmentRequest<'_>,
) -> Result<(PoseidonTranscript, Vec<Ext3>), PcsTranscriptError> {
    if request.witness_roots.len() != request.root_challenge_draws.len() {
        return Err(PcsTranscriptError::RootChallengeDrawMismatch {
            root_count: request.witness_roots.len(),
            draw_count: request.root_challenge_draws.len(),
        });
    }

    let mut transcript = PoseidonTranscript::new(request.arity)?;
    let mut challenges = Vec::new();
    transcript.put(&request.constant_root);

    if !request.public_values.is_empty() {
        absorb_commit_values(
            &mut transcript,
            request.arity,
            request.hash_values,
            request.public_values,
        )?;
    }

    for (stage_index, (root, draw_count)) in request
        .witness_roots
        .iter()
        .zip(request.root_challenge_draws.iter())
        .enumerate()
    {
        let stage =
            u32::try_from(stage_index + 1).map_err(|_| PcsTranscriptError::LengthOverflow)?;
        transcript.put(root);
        absorb_transcript_stage_unit_values(
            &mut transcript,
            stage,
            request.unit_value_map,
            request.unit_values,
        )?;
        draw_transcript_fields(&mut transcript, *draw_count, &mut challenges);
    }

    draw_transcript_fields(
        &mut transcript,
        request.evaluation_challenge_draws,
        &mut challenges,
    );
    if !request.evaluation_values.is_empty() {
        let values = flatten_extension_values_for_transcript(request.evaluation_values);
        absorb_commit_values(&mut transcript, request.arity, request.hash_values, &values)?;
    }

    absorb_binding_segments(&mut transcript, request.binding_segments)?;

    Ok((transcript, challenges))
}

fn absorb_transcript_stage_unit_values(
    transcript: &mut PoseidonTranscript,
    stage: u32,
    value_map: &[StageValue],
    values: &[Felt],
) -> Result<(), PcsTranscriptError> {
    let mut offset = 0_usize;
    for (value_index, value) in value_map.iter().enumerate() {
        let width = if value.stage == 1 { 1 } else { 3 };
        let end = offset
            .checked_add(width)
            .ok_or(PcsTranscriptError::LengthOverflow)?;
        if end > values.len() {
            return Err(PcsTranscriptError::UnitValueOutOfRange {
                value_index,
                offset,
                width,
                len: values.len(),
            });
        }
        if value.stage == stage && value.stage > 1 {
            transcript.put(&values[offset..end]);
        }
        offset = end;
    }
    Ok(())
}

fn draw_transcript_fields(transcript: &mut PoseidonTranscript, count: usize, out: &mut Vec<Ext3>) {
    for _ in 0..count {
        out.push(transcript.get_field());
    }
}

fn flatten_extension_values_for_transcript(values: &[Ext3]) -> Vec<Felt> {
    values
        .iter()
        .flat_map(|value| [value.c0, value.c1, value.c2])
        .collect()
}

fn ordered_opening_layers(
    fri: &PcsFriOpeningUnitSegment,
    expected_count: usize,
) -> Result<Vec<&PcsFriOpeningLayerSegment>, PcsFriOpeningFoldError> {
    let mut layers = Vec::with_capacity(expected_count);
    for layer_index in 0..expected_count {
        let layer_index_u32 =
            u32::try_from(layer_index).map_err(|_| PcsFriOpeningFoldError::LengthOverflow)?;
        let layer = fri
            .layers
            .iter()
            .find(|layer| layer.layer_index == layer_index_u32)
            .ok_or(PcsFriOpeningFoldError::MissingLayer {
                layer_index: layer_index_u32,
            })?;
        layers.push(layer);
    }
    Ok(layers)
}

fn domain_size(bits: u32) -> Result<u64, PcsFriOpeningFoldError> {
    1_u64
        .checked_shl(bits)
        .ok_or(PcsFriOpeningFoldError::UnsupportedDomainBits { bits })
}

fn convert_ext(values: [u64; 3]) -> Result<Ext3, PcsFriOpeningFoldError> {
    Ok(Ext3::new(
        convert_felt(values[0])?,
        convert_felt(values[1])?,
        convert_felt(values[2])?,
    ))
}

fn convert_felt(value: u64) -> Result<Felt, PcsFriOpeningFoldError> {
    Felt::from_canonical(value).map_err(|error| match error {
        FieldError::NonCanonical { value } => PcsFriOpeningFoldError::NonCanonicalField { value },
    })
}

fn group_fri_layer_values(
    layer_index: usize,
    polynomial: &[Ext3],
    output_size: usize,
    folding_factor: usize,
) -> Result<Vec<Vec<Ext3>>, PcsFriOpeningBuildError> {
    let expected = output_size
        .checked_mul(folding_factor)
        .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
    if polynomial.len() != expected {
        return Err(PcsFriOpeningBuildError::PolynomialLengthMismatch {
            layer_index,
            expected,
            found: polynomial.len(),
        });
    }

    let mut grouped = Vec::with_capacity(output_size);
    for row in 0..output_size {
        let mut values = Vec::with_capacity(folding_factor);
        for slot in 0..folding_factor {
            let index = slot
                .checked_mul(output_size)
                .and_then(|offset| offset.checked_add(row))
                .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
            values.push(polynomial[index]);
        }
        grouped.push(values);
    }
    Ok(grouped)
}

fn build_domain_size(bits: u32) -> Result<usize, PcsFriOpeningBuildError> {
    1_usize
        .checked_shl(bits)
        .ok_or(PcsFriOpeningBuildError::UnsupportedDomainBits { bits })
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

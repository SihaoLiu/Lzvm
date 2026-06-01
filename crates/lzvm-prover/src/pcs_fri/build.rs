use lzvm_artifacts::pcs_fri_segment::{
    PcsFriOpeningLayerSegment, PcsFriOpeningQuerySegment, PcsFriOpeningUnitSegment,
};
use lzvm_artifacts::setup_info::StageValue;
use lzvm_field::{Ext3, Felt, PoseidonTranscript};

use super::errors::{PcsFriOpeningBuildError, PcsFriTranscriptCommitmentError};
use super::fold::verify_fri_fold;
use super::merkle;
use super::requests::{
    PcsFriOpeningBuildRequest, PcsFriTranscriptCommitmentRequest, PcsFriTranscriptCommitments,
};
use crate::pcs_transcript::{absorb_binding_segments, absorb_commit_values, PcsTranscriptError};
use crate::ProveUnitSchedule;

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
        trace_instance_index: request.trace_instance_index,
        layers,
        final_polynomial: current.iter().map(|value| value.to_u64s()).collect(),
    })
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

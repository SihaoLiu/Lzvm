use std::time::Instant;

use lzvm_artifacts::pcs_fri_segment::{
    PcsFriOpeningLayerSegment, PcsFriOpeningQuerySegment, PcsFriOpeningUnitSegment,
};
use lzvm_field::{Ext3, Felt};

use super::errors::{PcsFriOpeningBuildError, PcsFriTranscriptCommitmentError};
use super::fold::verify_fri_fold;
use super::merkle;
use super::requests::{
    PcsFriOpeningBuildRequest, PcsFriOpeningBuildTiming, PcsFriTranscriptCommitmentRequest,
    PcsFriTranscriptCommitments, PcsFriTranscriptLayerMaterial,
};
use crate::pcs_transcript::{
    absorb_commit_values, build_pcs_transcript_prefix, PcsTranscriptPrefixInputs,
};
use crate::ProveUnitSchedule;

pub fn build_pcs_fri_transcript_commitments(
    schedule: &ProveUnitSchedule,
    request: PcsFriTranscriptCommitmentRequest<'_>,
) -> Result<PcsFriTranscriptCommitments, PcsFriTranscriptCommitmentError> {
    build_pcs_fri_transcript_commitments_with_timing(schedule, request, None)
}

pub fn build_pcs_fri_transcript_commitments_with_timing(
    schedule: &ProveUnitSchedule,
    request: PcsFriTranscriptCommitmentRequest<'_>,
    mut timing: Option<&mut PcsFriOpeningBuildTiming>,
) -> Result<PcsFriTranscriptCommitments, PcsFriTranscriptCommitmentError> {
    let unit_started = timing.as_ref().map(|_| Instant::now());
    if schedule.fri_layers.is_empty() {
        return Err(PcsFriOpeningBuildError::EmptyFriLayers.into());
    }

    let (mut transcript, mut challenges) =
        build_pcs_transcript_prefix(PcsTranscriptPrefixInputs {
            arity: request.arity,
            hash_values: request.hash_values,
            constant_root: request.constant_root,
            public_values: request.public_values,
            witness_roots: request.witness_roots,
            root_challenge_draws: request.root_challenge_draws,
            unit_value_map: request.unit_value_map,
            unit_values: request.unit_values,
            evaluation_values: request.evaluation_values,
            evaluation_challenge_draws: request.evaluation_challenge_draws,
            binding_segments: request.binding_segments,
        })?;
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
    let mut layer_materials = Vec::with_capacity(schedule.fri_layers.len());
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
        let tree = record_fri_transcript_duration(
            timing.as_deref_mut(),
            |timing, duration| timing.add_transcript_layer_tree(duration),
            || {
                merkle::build_fri_layer_tree(
                    &grouped_values,
                    arity,
                    schedule.last_level_verification,
                )
            },
        )?;
        layer_roots.push(tree.root);
        transcript.put(&tree.root);
        let challenge = transcript.get_field();
        challenges.push(challenge);

        let next = record_fri_transcript_duration(
            timing.as_deref_mut(),
            |timing, duration| timing.add_transcript_fold_work(duration),
            || {
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
                Ok::<Vec<Ext3>, PcsFriTranscriptCommitmentError>(next)
            },
        )?;
        layer_materials.push(PcsFriTranscriptLayerMaterial {
            grouped_values,
            tree,
        });
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

    if let (Some(timing), Some(started)) = (timing, unit_started) {
        timing.add_transcript_unit_build(started.elapsed());
    }
    Ok(PcsFriTranscriptCommitments {
        challenges,
        layer_roots,
        final_polynomial: current,
        final_query_challenge,
        layer_materials,
    })
}

fn record_fri_transcript_duration<T, E>(
    timing: Option<&mut PcsFriOpeningBuildTiming>,
    record: impl FnOnce(&mut PcsFriOpeningBuildTiming, std::time::Duration),
    build: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let Some(timing) = timing else {
        return build();
    };
    let started = Instant::now();
    let result = build();
    record(timing, started.elapsed());
    result
}

pub fn build_pcs_fri_opening_unit(
    schedule: &ProveUnitSchedule,
    request: PcsFriOpeningBuildRequest<'_>,
) -> Result<PcsFriOpeningUnitSegment, PcsFriOpeningBuildError> {
    build_pcs_fri_opening_unit_with_timing(schedule, request, None)
}

pub fn build_pcs_fri_opening_unit_with_timing(
    schedule: &ProveUnitSchedule,
    request: PcsFriOpeningBuildRequest<'_>,
    mut timing: Option<&mut PcsFriOpeningBuildTiming>,
) -> Result<PcsFriOpeningUnitSegment, PcsFriOpeningBuildError> {
    let unit_started = timing.as_ref().map(|_| Instant::now());
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
        let tree = record_fri_opening_duration(
            timing.as_deref_mut(),
            |timing, duration| timing.add_layer_tree(duration),
            || {
                merkle::build_fri_layer_tree(
                    &grouped_values,
                    arity,
                    schedule.last_level_verification,
                )
            },
        )?;
        let layer_index_u32 =
            u32::try_from(layer_index).map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
        let output_size_u64 =
            u64::try_from(output_size).map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
        let queries = record_fri_opening_duration(
            timing.as_deref_mut(),
            |timing, duration| timing.add_query_work(duration, request.query_rows.len()),
            || {
                request
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
                    .collect::<Result<Vec<_>, PcsFriOpeningBuildError>>()
            },
        )?;

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
        let next = record_fri_opening_duration(
            timing.as_deref_mut(),
            |timing, duration| timing.add_fold_work(duration),
            || {
                let mut next = Vec::with_capacity(output_size);
                for (row_index, values) in grouped_values.iter().enumerate() {
                    next.push(verify_fri_fold(
                        schedule.extended_domain_bits,
                        layer.output_bits,
                        layer.input_bits,
                        challenge,
                        u64::try_from(row_index)
                            .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?,
                        values,
                    )?);
                }
                Ok::<Vec<Ext3>, PcsFriOpeningBuildError>(next)
            },
        )?;
        current = next;
        current_bits = layer.output_bits;
    }

    if current_bits != schedule.final_layer_bits {
        return Err(PcsFriOpeningBuildError::FinalLayerMismatch {
            expected: schedule.final_layer_bits,
            found: current_bits,
        });
    }

    let segment = PcsFriOpeningUnitSegment {
        unit_index: request.unit_index,
        trace_instance_index: request.trace_instance_index,
        layers,
        final_polynomial: current.iter().map(|value| value.to_u64s()).collect(),
    };
    if let (Some(timing), Some(started)) = (timing, unit_started) {
        timing.add_unit_build(started.elapsed());
    }
    Ok(segment)
}

pub(crate) fn build_pcs_fri_opening_unit_from_transcript_commitments_with_timing(
    schedule: &ProveUnitSchedule,
    unit_index: u32,
    trace_instance_index: u32,
    query_rows: &[u64],
    commitments: &PcsFriTranscriptCommitments,
    mut timing: Option<&mut PcsFriOpeningBuildTiming>,
) -> Result<PcsFriOpeningUnitSegment, PcsFriOpeningBuildError> {
    let unit_started = timing.as_ref().map(|_| Instant::now());
    if schedule.fri_layers.is_empty() {
        return Err(PcsFriOpeningBuildError::EmptyFriLayers);
    }

    let query_count = usize::try_from(schedule.query_count)
        .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
    if query_rows.len() != query_count {
        return Err(PcsFriOpeningBuildError::QueryRowCountMismatch {
            expected: query_count,
            found: query_rows.len(),
        });
    }
    if commitments.layer_materials.len() != schedule.fri_layers.len() {
        return Err(PcsFriOpeningBuildError::LayerCountMismatch {
            expected: schedule.fri_layers.len(),
            found: commitments.layer_materials.len(),
        });
    }
    if commitments.layer_roots.len() != schedule.fri_layers.len() {
        return Err(PcsFriOpeningBuildError::LayerCountMismatch {
            expected: schedule.fri_layers.len(),
            found: commitments.layer_roots.len(),
        });
    }

    let mut current_bits = schedule.fri_layers[0].input_bits;
    let mut layers = Vec::with_capacity(schedule.fri_layers.len());
    for (layer_index, (layer, material)) in schedule
        .fri_layers
        .iter()
        .zip(commitments.layer_materials.iter())
        .enumerate()
    {
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
        if material.grouped_values.len() != output_size {
            return Err(PcsFriOpeningBuildError::PolynomialLengthMismatch {
                layer_index,
                expected: output_size,
                found: material.grouped_values.len(),
            });
        }
        if let Some(found) = material
            .grouped_values
            .iter()
            .map(Vec::len)
            .find(|len| *len != folding_factor)
        {
            return Err(PcsFriOpeningBuildError::FoldingFactorMismatch {
                layer_index,
                expected: folding_factor,
                found,
            });
        }
        if material.tree.root != commitments.layer_roots[layer_index] {
            return Err(PcsFriOpeningBuildError::LayerRootMismatch { layer_index });
        }

        let layer_index_u32 =
            u32::try_from(layer_index).map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
        let output_size_u64 =
            u64::try_from(output_size).map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
        let queries = record_fri_opening_duration(
            timing.as_deref_mut(),
            |timing, duration| timing.add_query_work(duration, query_rows.len()),
            || {
                query_rows
                    .iter()
                    .map(|query_row| {
                        let row_index = *query_row % output_size_u64;
                        let row_index_usize = usize::try_from(row_index)
                            .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
                        let values = material.grouped_values[row_index_usize]
                            .iter()
                            .map(|value| value.to_u64s())
                            .collect();
                        let siblings = material.tree.query_siblings(row_index_usize)?;
                        Ok(PcsFriOpeningQuerySegment {
                            row_index,
                            values,
                            siblings,
                        })
                    })
                    .collect::<Result<Vec<_>, PcsFriOpeningBuildError>>()
            },
        )?;

        layers.push(PcsFriOpeningLayerSegment {
            layer_index: layer_index_u32,
            root: merkle::digest_to_u64s(material.tree.root),
            last_level: material
                .tree
                .last_level
                .iter()
                .copied()
                .map(merkle::digest_to_u64s)
                .collect(),
            queries,
        });
        current_bits = layer.output_bits;
    }

    if current_bits != schedule.final_layer_bits {
        return Err(PcsFriOpeningBuildError::FinalLayerMismatch {
            expected: schedule.final_layer_bits,
            found: current_bits,
        });
    }
    let final_len = build_domain_size(schedule.final_layer_bits)?;
    if commitments.final_polynomial.len() != final_len {
        return Err(PcsFriOpeningBuildError::PolynomialLengthMismatch {
            layer_index: schedule.fri_layers.len(),
            expected: final_len,
            found: commitments.final_polynomial.len(),
        });
    }

    let segment = PcsFriOpeningUnitSegment {
        unit_index,
        trace_instance_index,
        layers,
        final_polynomial: commitments
            .final_polynomial
            .iter()
            .map(|value| value.to_u64s())
            .collect(),
    };
    if let (Some(timing), Some(started)) = (timing, unit_started) {
        timing.add_unit_build(started.elapsed());
    }
    Ok(segment)
}

fn record_fri_opening_duration<T, E>(
    timing: Option<&mut PcsFriOpeningBuildTiming>,
    record: impl FnOnce(&mut PcsFriOpeningBuildTiming, std::time::Duration),
    build: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let Some(timing) = timing else {
        return build();
    };
    let started = Instant::now();
    let result = build();
    record(timing, started.elapsed());
    result
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

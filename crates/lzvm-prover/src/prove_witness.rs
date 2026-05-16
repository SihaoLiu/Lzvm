mod types;

pub use types::*;

use std::collections::{BTreeMap, BTreeSet};

use lzvm_artifacts::constant_opening_segment::{
    encode_constant_opening_segment, ConstantOpeningLevelSegment, ConstantOpeningQuerySegment,
    ConstantOpeningSegment, ConstantOpeningUnitSegment, CONSTANT_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::constant_tree::read_constant_tree_file;
use lzvm_artifacts::key_directory::KeyDirectoryCatalog;
use lzvm_artifacts::pcs_evaluation_segment::{
    encode_pcs_evaluation_segment, PcsEvaluationSegment, PcsEvaluationUnitSegment,
    PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::pcs_material_segment::{
    encode_pcs_material_manifest_segment, PcsMaterialManifestSegment, PcsMaterialManifestUnit,
    PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
use lzvm_artifacts::pcs_nonce_segment::{
    encode_pcs_query_nonce_segment, parse_pcs_query_nonce_segment, PcsQueryNonceSegment,
    PCS_QUERY_NONCE_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::{
    encode_pcs_query_plan_segment, parse_pcs_query_plan_segment, PcsQueryPlanSegment,
    PcsQueryPlanSegmentError, PcsQueryPlanUnit, PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_opening_segment::{
    encode_witness_opening_segment, WitnessOpeningLevelSegment, WitnessOpeningQuerySegment,
    WitnessOpeningSegment, WitnessOpeningStageSegment, WitnessOpeningUnitSegment,
    WITNESS_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, parse_witness_commitment_segment, WitnessCommitmentSegment,
    WitnessCommitmentStageSegment, WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_field::{Ext3, Felt};
use sha2::{Digest, Sha256};

use crate::constant_tree_opening::open_constant_tree_row;
#[cfg(not(feature = "cuda"))]
use crate::pcs_challenge::find_query_nonce;
#[cfg(feature = "cuda")]
use crate::pcs_challenge::find_query_nonce_cuda_with_streams;
use crate::pcs_challenge::{derive_fri_queries, verify_query_nonce};
use crate::pcs_transcript::{
    derive_pcs_final_query_challenge_from_segments, PcsTranscriptSegmentInputs,
};
use crate::witness_commitment::open_witness_stage_commitment;
use crate::witness_execution::ProveWitnessCommitments;
use crate::ProveSchedule;

pub fn build_witness_commitment_segment(
    output: &ProveWitnessCommitments,
) -> Result<ProofSegment, ProveWitnessSegmentError> {
    let unit_index =
        u32::try_from(output.unit_index()).map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
    let id = WITNESS_COMMITMENT_SEGMENT_BASE_ID
        .checked_add(unit_index)
        .ok_or(ProveWitnessSegmentError::LengthOverflow)?;
    let mut stages = Vec::with_capacity(output.stage_commitments().stage_count());
    for commitment in output.stage_commitments().commitments() {
        let stage_index = u32::try_from(commitment.stage_index())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
        let arity = u32::try_from(commitment.arity())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
        let tree_byte_count = u64::try_from(commitment.tree_bytes().len())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
        stages.push(WitnessCommitmentStageSegment {
            stage_index,
            arity,
            root: commitment.root().map(|value| value.to_u64()),
            tree_byte_count,
            tree_digest: Sha256::digest(commitment.tree_bytes()).into(),
        });
    }

    let segment = WitnessCommitmentSegment {
        unit_index,
        input_byte_count: u64::try_from(output.input_byte_count())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?,
        trace_rows: u64::try_from(output.trace_row_count())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?,
        trace_columns: u64::try_from(output.trace_column_count())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?,
        stages,
    };
    Ok(ProofSegment {
        id,
        data: encode_witness_commitment_segment(&segment)?,
    })
}

pub fn build_pcs_material_manifest_segment(
    schedule: &ProveSchedule,
) -> Result<ProofSegment, ProvePcsMaterialSegmentError> {
    let mut units = Vec::with_capacity(schedule.units.len());
    for (unit_index, unit) in schedule.units.iter().enumerate() {
        let unit_index_u32 = u32::try_from(unit_index)
            .map_err(|_| ProvePcsMaterialSegmentError::UnitIndexOverflow { unit_index })?;
        units.push(PcsMaterialManifestUnit {
            unit_index: unit_index_u32,
            plan_digest: unit.pcs_material_plan_digest.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            fixed_column_digest: unit.pcs_material_fixed_column_digest.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            constant_tree_digest: unit.pcs_material_constant_tree_digest.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            constant_tree_root: unit.pcs_material_constant_tree_root.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            fixed_byte_count: unit.pcs_material_fixed_byte_count.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            constant_tree_byte_count: unit.pcs_material_constant_tree_byte_count.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            leaf_byte_count: unit.pcs_material_leaf_byte_count.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            node_byte_count: unit.pcs_material_node_byte_count.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
        });
    }
    let manifest = PcsMaterialManifestSegment { units };
    Ok(ProofSegment {
        id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
        data: encode_pcs_material_manifest_segment(&manifest)?,
    })
}

pub fn build_pcs_evaluation_segment(
    schedule: &ProveSchedule,
    values: &[ProvePcsEvaluationValues],
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
            values: input.values.iter().copied().map(Ext3::to_u64s).collect(),
        });
    }
    units.sort_by_key(|unit| unit.unit_index);

    let segment = PcsEvaluationSegment { units };
    Ok(ProofSegment {
        id: PCS_EVALUATION_SEGMENT_ID,
        data: encode_pcs_evaluation_segment(&segment)?,
    })
}

pub fn build_pcs_query_plan_segment(
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    material_segment: &ProofSegment,
    witness_segments: &[ProofSegment],
) -> Result<ProofSegment, ProvePcsQueryPlanSegmentError> {
    build_pcs_query_plan_segment_with_bindings(
        schedule,
        public_values_hash,
        material_segment,
        witness_segments,
        &[],
    )
}

pub fn build_pcs_query_plan_segment_with_bindings(
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    material_segment: &ProofSegment,
    witness_segments: &[ProofSegment],
    binding_segments: &[ProofSegment],
) -> Result<ProofSegment, ProvePcsQueryPlanSegmentError> {
    let witness_segments = sorted_witness_commitment_segments(witness_segments)?;

    let mut hasher = Sha256::new();
    hasher.update(b"lzvm-pcs-query-plan-v1");
    hasher.update(schedule.setup_hash);
    hasher.update(public_values_hash);
    hash_proof_segment(&mut hasher, material_segment)?;
    for segment in &witness_segments {
        hash_proof_segment(&mut hasher, segment)?;
    }
    for segment in binding_segments {
        hash_proof_segment(&mut hasher, segment)?;
    }
    let seed: [u8; 32] = hasher.finalize().into();

    let query_units = collect_witness_query_units(schedule, &witness_segments)?;
    let mut units = Vec::with_capacity(query_units.len());
    for (unit_index_u32, unit) in query_units {
        units.push(PcsQueryPlanUnit {
            unit_index: unit_index_u32,
            queries: derive_unit_queries(
                &seed,
                unit_index_u32,
                unit.query_count,
                unit.extended_domain_size,
            )?,
        });
    }

    let query_plan = PcsQueryPlanSegment { units };
    Ok(ProofSegment {
        id: PCS_QUERY_PLAN_SEGMENT_ID,
        data: encode_pcs_query_plan_segment(&query_plan)?,
    })
}

pub fn build_pcs_query_nonce_segment(
    schedule: &ProveSchedule,
    challenge: Ext3,
) -> Result<ProofSegment, ProvePcsQueryPlanSegmentError> {
    build_pcs_query_nonce_segment_with_streams(schedule, challenge, 1)
}

pub fn build_pcs_query_nonce_segment_with_streams(
    schedule: &ProveSchedule,
    challenge: Ext3,
    max_streams: usize,
) -> Result<ProofSegment, ProvePcsQueryPlanSegmentError> {
    let bits = schedule
        .units
        .iter()
        .map(|unit| unit.proof_of_work_bits)
        .max()
        .unwrap_or(0);
    let nonce = find_query_nonce_with_available_backend(challenge, bits, max_streams)?;
    let segment = PcsQueryNonceSegment {
        nonce: nonce.to_u64(),
    };
    Ok(ProofSegment {
        id: PCS_QUERY_NONCE_SEGMENT_ID,
        data: encode_pcs_query_nonce_segment(&segment)?,
    })
}

pub fn build_pcs_query_nonce_segment_from_transcript_segments(
    schedule: &ProveSchedule,
    input: PcsTranscriptSegmentInputs<'_>,
) -> Result<ProofSegment, ProvePcsQueryPlanSegmentError> {
    let challenge = derive_pcs_final_query_challenge_from_segments(input)?;
    build_pcs_query_nonce_segment(schedule, challenge)
}

pub fn build_pcs_query_plan_segment_from_transcript_segments(
    schedule: &ProveSchedule,
    witness_segments: &[ProofSegment],
    input: PcsTranscriptSegmentInputs<'_>,
    nonce_segment: &ProofSegment,
) -> Result<ProofSegment, ProvePcsQueryPlanSegmentError> {
    if nonce_segment.id != PCS_QUERY_NONCE_SEGMENT_ID {
        return Err(ProvePcsQueryPlanSegmentError::InvalidNonceSegmentId {
            segment_id: nonce_segment.id,
        });
    }

    let challenge = derive_pcs_final_query_challenge_from_segments(input)?;
    let nonce = Felt::from_u64(parse_pcs_query_nonce_segment(&nonce_segment.data)?.nonce);
    build_pcs_query_plan_segment_from_challenge(schedule, witness_segments, challenge, nonce)
}

#[cfg(feature = "cuda")]
fn find_query_nonce_with_available_backend(
    challenge: Ext3,
    bits: u32,
    max_streams: usize,
) -> Result<Felt, ProvePcsQueryPlanSegmentError> {
    Ok(find_query_nonce_cuda_with_streams(
        challenge,
        bits,
        max_streams,
    )?)
}

#[cfg(not(feature = "cuda"))]
fn find_query_nonce_with_available_backend(
    challenge: Ext3,
    bits: u32,
    _max_streams: usize,
) -> Result<Felt, ProvePcsQueryPlanSegmentError> {
    Ok(find_query_nonce(challenge, bits)?)
}

pub fn build_pcs_query_plan_segment_from_challenge(
    schedule: &ProveSchedule,
    witness_segments: &[ProofSegment],
    challenge: Ext3,
    nonce: Felt,
) -> Result<ProofSegment, ProvePcsQueryPlanSegmentError> {
    let witness_segments = sorted_witness_commitment_segments(witness_segments)?;
    let query_units = collect_witness_query_units(schedule, &witness_segments)?;
    let mut units = Vec::with_capacity(query_units.len());
    for (unit_index_u32, unit) in query_units {
        let unit_index = usize::try_from(unit_index_u32)
            .map_err(|_| ProvePcsQueryPlanSegmentError::LengthOverflow)?;
        if !verify_query_nonce(challenge, nonce, unit.proof_of_work_bits)? {
            return Err(ProvePcsQueryPlanSegmentError::QueryNonceMismatch {
                unit_index,
                bits: unit.proof_of_work_bits,
            });
        }
        let arity = unit
            .transcript_arity
            .ok_or(ProvePcsQueryPlanSegmentError::MissingTranscriptArity { unit_index })?
            as usize;
        units.push(PcsQueryPlanUnit {
            unit_index: unit_index_u32,
            queries: derive_fri_queries(
                arity,
                challenge,
                nonce,
                unit.query_count as usize,
                unit.extended_domain_bits,
            )?,
        });
    }

    let query_plan = PcsQueryPlanSegment { units };
    Ok(ProofSegment {
        id: PCS_QUERY_PLAN_SEGMENT_ID,
        data: encode_pcs_query_plan_segment(&query_plan)?,
    })
}

pub fn build_witness_opening_segment(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    output: &ProveWitnessCommitments,
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
    build_witness_opening_segment_from_query_plan(schedule, &query_plan, &[output])
}

pub fn build_witness_opening_segment_batch(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    outputs: &[&ProveWitnessCommitments],
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
    build_witness_opening_segment_from_query_plan(schedule, &query_plan, outputs)
}

fn build_witness_opening_segment_from_query_plan(
    schedule: &ProveSchedule,
    query_plan: &PcsQueryPlanSegment,
    outputs: &[&ProveWitnessCommitments],
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let mut outputs_by_unit = BTreeMap::new();
    for output in outputs {
        let unit_index_u32 = u32::try_from(output.unit_index()).map_err(|_| {
            ProveWitnessOpeningSegmentError::UnitIndexOverflow {
                unit_index: output.unit_index(),
            }
        })?;
        if outputs_by_unit.insert(unit_index_u32, *output).is_some() {
            return Err(ProveWitnessOpeningSegmentError::DuplicateOutputUnit {
                unit_index: output.unit_index(),
            });
        }
    }

    let query_units = query_plan
        .units
        .iter()
        .map(|unit| unit.unit_index)
        .collect::<BTreeSet<_>>();
    for unit_index_u32 in outputs_by_unit.keys() {
        if !query_units.contains(unit_index_u32) {
            return Err(ProveWitnessOpeningSegmentError::MissingQueryUnit {
                unit_index: *unit_index_u32 as usize,
            });
        }
    }

    let mut units = Vec::with_capacity(query_plan.units.len());
    for query_unit in &query_plan.units {
        let unit_index = query_unit.unit_index as usize;
        let output = outputs_by_unit
            .get(&query_unit.unit_index)
            .ok_or(ProveWitnessOpeningSegmentError::MissingOutputUnit { unit_index })?;
        units.push(build_witness_opening_unit_segment(
            schedule, query_unit, output,
        )?);
    }

    let segment = WitnessOpeningSegment { units };
    Ok(ProofSegment {
        id: WITNESS_OPENING_SEGMENT_ID,
        data: encode_witness_opening_segment(&segment)?,
    })
}

fn build_witness_opening_unit_segment(
    schedule: &ProveSchedule,
    query_unit: &PcsQueryPlanUnit,
    output: &ProveWitnessCommitments,
) -> Result<WitnessOpeningUnitSegment, ProveWitnessOpeningSegmentError> {
    let unit_index_u32 = u32::try_from(output.unit_index()).map_err(|_| {
        ProveWitnessOpeningSegmentError::UnitIndexOverflow {
            unit_index: output.unit_index(),
        }
    })?;
    if query_unit.unit_index != unit_index_u32 {
        return Err(ProveWitnessOpeningSegmentError::MissingQueryUnit {
            unit_index: output.unit_index(),
        });
    }
    let unit = schedule.units.get(output.unit_index()).ok_or(
        ProveWitnessOpeningSegmentError::UnitIndexOutOfRange {
            unit_index: output.unit_index(),
            unit_count: schedule.units.len(),
        },
    )?;
    let mut queries = Vec::with_capacity(query_unit.queries.len());
    for row_index in &query_unit.queries {
        let mut stages = Vec::with_capacity(output.stage_commitments().stage_count());
        for commitment in output.stage_commitments().commitments() {
            let stage_index = commitment.stage_index();
            let width = unit
                .stage_commit_widths
                .get(stage_index.checked_sub(1).ok_or(
                    ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                        stage_index,
                        stage_count: unit.stage_commit_widths.len(),
                    },
                )?)
                .ok_or(ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                    stage_index,
                    stage_count: unit.stage_commit_widths.len(),
                })?;
            let opening = open_witness_stage_commitment(
                commitment,
                *row_index,
                unit.extended_domain_size,
                usize::try_from(*width).map_err(|_| {
                    ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                        stage_index,
                        stage_count: unit.stage_commit_widths.len(),
                    }
                })?,
            )?;
            stages.push(WitnessOpeningStageSegment {
                stage_index: u32::try_from(stage_index).map_err(|_| {
                    ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                        stage_index,
                        stage_count: unit.stage_commit_widths.len(),
                    }
                })?,
                values: opening
                    .values()
                    .iter()
                    .map(|value| value.to_u64())
                    .collect(),
                siblings: opening
                    .siblings()
                    .iter()
                    .map(|level| WitnessOpeningLevelSegment {
                        siblings: level
                            .iter()
                            .map(|digest| digest.map(|value| value.to_u64()))
                            .collect(),
                    })
                    .collect(),
            });
        }
        queries.push(WitnessOpeningQuerySegment {
            row_index: *row_index,
            stages,
        });
    }

    Ok(WitnessOpeningUnitSegment {
        unit_index: unit_index_u32,
        queries,
    })
}

pub fn build_constant_opening_segment(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
) -> Result<ProofSegment, ProveConstantOpeningSegmentError> {
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
    let mut units = Vec::with_capacity(query_plan.units.len());
    for query_unit in &query_plan.units {
        let unit_index = usize::try_from(query_unit.unit_index).map_err(|_| {
            ProveConstantOpeningSegmentError::UnitIndexOverflow {
                unit_index: query_unit.unit_index,
            }
        })?;
        let schedule_unit = schedule.units.get(unit_index).ok_or(
            ProveConstantOpeningSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let catalog_unit = catalog.units.get(unit_index).ok_or(
            ProveConstantOpeningSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: catalog.units.len(),
            },
        )?;
        let tree = read_constant_tree_file(
            &catalog_unit.paths.constant_tree,
            &catalog_unit.metadata.setup,
        )
        .map_err(|source| ProveConstantOpeningSegmentError::ConstantTree { unit_index, source })?;
        let arity = usize::try_from(schedule_unit.merkle_tree_arity).map_err(|_| {
            ProveConstantOpeningSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: schedule.units.len(),
            }
        })?;
        let mut queries = Vec::with_capacity(query_unit.queries.len());
        for row_index in &query_unit.queries {
            let opening = open_constant_tree_row(&tree, *row_index, arity)?;
            queries.push(ConstantOpeningQuerySegment {
                row_index: *row_index,
                values: opening
                    .values()
                    .iter()
                    .map(|value| value.to_u64())
                    .collect(),
                siblings: opening
                    .siblings()
                    .iter()
                    .map(|level| ConstantOpeningLevelSegment {
                        siblings: level
                            .iter()
                            .map(|digest| digest.map(|value| value.to_u64()))
                            .collect(),
                    })
                    .collect(),
            });
        }
        units.push(ConstantOpeningUnitSegment {
            unit_index: query_unit.unit_index,
            queries,
        });
    }

    let segment = ConstantOpeningSegment { units };
    Ok(ProofSegment {
        id: CONSTANT_OPENING_SEGMENT_ID,
        data: encode_constant_opening_segment(&segment)?,
    })
}

fn sorted_witness_commitment_segments(
    witness_segments: &[ProofSegment],
) -> Result<Vec<ProofSegment>, ProvePcsQueryPlanSegmentError> {
    if witness_segments.is_empty() {
        return Err(ProvePcsQueryPlanSegmentError::MissingWitnessSegments);
    }
    let mut out = witness_segments.to_vec();
    out.sort_by_key(|segment| segment.id);
    Ok(out)
}

fn collect_witness_query_units<'a>(
    schedule: &'a ProveSchedule,
    witness_segments: &[ProofSegment],
) -> Result<Vec<(u32, &'a crate::ProveUnitSchedule)>, ProvePcsQueryPlanSegmentError> {
    let mut units = Vec::with_capacity(witness_segments.len());
    let mut seen_units = BTreeSet::new();
    for segment in witness_segments {
        let unit_index_u32 = segment
            .id
            .checked_sub(WITNESS_COMMITMENT_SEGMENT_BASE_ID)
            .ok_or(ProvePcsQueryPlanSegmentError::LengthOverflow)?;
        let unit_index = usize::try_from(unit_index_u32)
            .map_err(|_| ProvePcsQueryPlanSegmentError::LengthOverflow)?;
        let unit = schedule.units.get(unit_index).ok_or(
            ProvePcsQueryPlanSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let witness = parse_witness_commitment_segment(&segment.data).map_err(|source| {
            ProvePcsQueryPlanSegmentError::InvalidWitnessSegment { unit_index, source }
        })?;
        if witness.unit_index != unit_index_u32 {
            return Err(ProvePcsQueryPlanSegmentError::WitnessUnitMismatch {
                segment_unit_index: unit_index_u32,
                payload_unit_index: witness.unit_index,
            });
        }
        if !seen_units.insert(unit_index_u32) {
            return Err(ProvePcsQueryPlanSegmentError::Segment(
                PcsQueryPlanSegmentError::DuplicateUnitIndex {
                    unit_index: unit_index_u32,
                },
            ));
        }
        units.push((unit_index_u32, unit));
    }
    Ok(units)
}

fn derive_unit_queries(
    seed: &[u8; 32],
    unit_index: u32,
    query_count: u32,
    domain_size: u64,
) -> Result<Vec<u64>, ProvePcsQueryPlanSegmentError> {
    if u64::from(query_count) > domain_size {
        return Err(ProvePcsQueryPlanSegmentError::QueryCountExceedsDomain {
            unit_index: unit_index as usize,
            query_count,
            domain_size,
        });
    }
    let mut queries = Vec::with_capacity(query_count as usize);
    let mut seen = BTreeSet::new();
    let mask = domain_size
        .checked_sub(1)
        .ok_or(ProvePcsQueryPlanSegmentError::LengthOverflow)?;
    let mut draw = 0_u64;
    while queries.len() < query_count as usize {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        hasher.update(unit_index.to_le_bytes());
        hasher.update(draw.to_le_bytes());
        let digest: [u8; 32] = hasher.finalize().into();
        let raw = u64::from_le_bytes(digest[..8].try_into().expect("slice length checked"));
        let query = raw & mask;
        if seen.insert(query) {
            queries.push(query);
        }
        draw = draw
            .checked_add(1)
            .ok_or(ProvePcsQueryPlanSegmentError::LengthOverflow)?;
    }
    Ok(queries)
}

fn hash_proof_segment(
    hasher: &mut Sha256,
    segment: &ProofSegment,
) -> Result<(), ProvePcsQueryPlanSegmentError> {
    hasher.update(segment.id.to_le_bytes());
    let byte_count = u64::try_from(segment.data.len())
        .map_err(|_| ProvePcsQueryPlanSegmentError::LengthOverflow)?;
    hasher.update(byte_count.to_le_bytes());
    hasher.update(Sha256::digest(&segment.data));
    Ok(())
}

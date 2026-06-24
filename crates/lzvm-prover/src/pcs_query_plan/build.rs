use std::collections::BTreeSet;

use lzvm_artifacts::pcs_nonce_segment::{
    encode_pcs_query_nonce_segment, parse_pcs_query_nonce_segment, PcsQueryNonceSegment,
    PCS_QUERY_NONCE_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::{
    encode_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanSegmentError, PcsQueryPlanUnit,
    PCS_QUERY_PLAN_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_segment::{
    parse_witness_commitment_segment, witness_commitment_segment_identity,
};
use lzvm_field::{Ext3, Felt};
use sha2::{Digest, Sha256};

#[cfg(not(feature = "cuda"))]
use crate::pcs_challenge::find_query_nonce;
#[cfg(feature = "cuda")]
use crate::pcs_challenge::find_query_nonce_cuda_with_streams;
use crate::pcs_challenge::{derive_fri_queries, verify_query_nonce};
use crate::pcs_transcript::{
    derive_pcs_final_query_challenge_from_segments, PcsTranscriptSegmentInputs,
};
use crate::ProveSchedule;

use crate::pcs_query_plan::ProvePcsQueryPlanSegmentError;

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
    let witness_segment_refs = witness_segments.iter().collect::<Vec<_>>();
    build_pcs_query_plan_segment_with_binding_refs(
        schedule,
        public_values_hash,
        material_segment,
        &witness_segment_refs,
        binding_segments,
    )
}

pub(super) fn build_pcs_query_plan_segment_with_binding_refs(
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    material_segment: &ProofSegment,
    witness_segments: &[&ProofSegment],
    binding_segments: &[ProofSegment],
) -> Result<ProofSegment, ProvePcsQueryPlanSegmentError> {
    let witness_segments = sorted_witness_commitment_segment_refs(witness_segments)?;

    let mut hasher = Sha256::new();
    hasher.update(b"lzvm-pcs-query-plan-v1");
    hasher.update(schedule.setup_hash);
    hasher.update(public_values_hash);
    hash_proof_segment(&mut hasher, material_segment)?;
    let unit_count = u32::try_from(schedule.units.len())
        .map_err(|_| ProvePcsQueryPlanSegmentError::LengthOverflow)?;
    for segment in witness_segments.iter().copied() {
        hash_witness_commitment_segment_for_query_seed(&mut hasher, unit_count, segment)?;
    }
    for segment in binding_segments {
        hash_proof_segment(&mut hasher, segment)?;
    }
    let seed: [u8; 32] = hasher.finalize().into();

    let query_units = collect_witness_query_units(schedule, &witness_segments)?;
    let mut units = Vec::with_capacity(query_units.len());
    for (identity, unit) in query_units {
        units.push(PcsQueryPlanUnit {
            unit_index: identity.unit_index,
            trace_instance_index: identity.trace_instance_index,
            queries: derive_unit_queries(
                &seed,
                identity.unit_index,
                identity.trace_instance_index,
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
    if max_streams == 0 {
        return Err(ProvePcsQueryPlanSegmentError::InvalidStreamCount);
    }
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

    let input_unit_index = u32::try_from(input.unit_index)
        .map_err(|_| ProvePcsQueryPlanSegmentError::LengthOverflow)?;
    let witness_segments = sorted_witness_commitment_segments(witness_segments)?;
    let query_units = collect_witness_query_units(schedule, &witness_segments)?;
    if query_units.len() != 1 {
        return Err(
            ProvePcsQueryPlanSegmentError::TranscriptWitnessUnitCountMismatch {
                expected: 1,
                found: query_units.len(),
            },
        );
    }
    let witness_unit_index = query_units[0].0.unit_index;
    if witness_unit_index != input_unit_index {
        return Err(
            ProvePcsQueryPlanSegmentError::TranscriptWitnessUnitMismatch {
                input_unit_index,
                witness_unit_index,
            },
        );
    }

    let challenge = derive_pcs_final_query_challenge_from_segments(input)?;
    let nonce = Felt::from_u64(parse_pcs_query_nonce_segment(&nonce_segment.data)?.nonce);
    build_pcs_query_plan_segment_from_sorted_challenge(
        schedule,
        &witness_segments,
        challenge,
        nonce,
    )
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
    let witness_segment_refs = witness_segments.iter().collect::<Vec<_>>();
    build_pcs_query_plan_segment_from_challenge_refs(
        schedule,
        &witness_segment_refs,
        challenge,
        nonce,
    )
}

pub(super) fn build_pcs_query_plan_segment_from_challenge_refs(
    schedule: &ProveSchedule,
    witness_segments: &[&ProofSegment],
    challenge: Ext3,
    nonce: Felt,
) -> Result<ProofSegment, ProvePcsQueryPlanSegmentError> {
    let witness_segments = sorted_witness_commitment_segment_refs(witness_segments)?;
    build_pcs_query_plan_segment_from_sorted_challenge(
        schedule,
        &witness_segments,
        challenge,
        nonce,
    )
}

fn build_pcs_query_plan_segment_from_sorted_challenge(
    schedule: &ProveSchedule,
    witness_segments: &[&ProofSegment],
    challenge: Ext3,
    nonce: Felt,
) -> Result<ProofSegment, ProvePcsQueryPlanSegmentError> {
    let query_units = collect_witness_query_units(schedule, witness_segments)?;
    let mut units = Vec::with_capacity(query_units.len());
    for (identity, unit) in query_units {
        let unit_index = usize::try_from(identity.unit_index)
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
            unit_index: identity.unit_index,
            trace_instance_index: identity.trace_instance_index,
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

fn sorted_witness_commitment_segments(
    witness_segments: &[ProofSegment],
) -> Result<Vec<&ProofSegment>, ProvePcsQueryPlanSegmentError> {
    let witness_segment_refs = witness_segments.iter().collect::<Vec<_>>();
    sorted_witness_commitment_segment_refs(&witness_segment_refs)
}

fn sorted_witness_commitment_segment_refs<'a>(
    witness_segments: &[&'a ProofSegment],
) -> Result<Vec<&'a ProofSegment>, ProvePcsQueryPlanSegmentError> {
    if witness_segments.is_empty() {
        return Err(ProvePcsQueryPlanSegmentError::MissingWitnessSegments);
    }
    let mut out = witness_segments.iter().copied().collect::<Vec<_>>();
    out.sort_by_key(|segment| segment.id);
    Ok(out)
}

fn collect_witness_query_units<'a>(
    schedule: &'a ProveSchedule,
    witness_segments: &[&ProofSegment],
) -> Result<
    Vec<(
        lzvm_artifacts::witness_segment::WitnessCommitmentSegmentIdentity,
        &'a crate::ProveUnitSchedule,
    )>,
    ProvePcsQueryPlanSegmentError,
> {
    let mut units = Vec::with_capacity(witness_segments.len());
    let mut seen_units = BTreeSet::new();
    let unit_count = u32::try_from(schedule.units.len())
        .map_err(|_| ProvePcsQueryPlanSegmentError::LengthOverflow)?;
    for segment in witness_segments {
        let identity = witness_commitment_segment_identity(unit_count, segment.id)
            .map_err(|_| ProvePcsQueryPlanSegmentError::LengthOverflow)?
            .ok_or(ProvePcsQueryPlanSegmentError::LengthOverflow)?;
        let unit_index = usize::try_from(identity.unit_index)
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
        if witness.unit_index != identity.unit_index {
            return Err(ProvePcsQueryPlanSegmentError::WitnessUnitMismatch {
                segment_unit_index: identity.unit_index,
                payload_unit_index: witness.unit_index,
            });
        }
        if !seen_units.insert((identity.unit_index, identity.trace_instance_index)) {
            return Err(ProvePcsQueryPlanSegmentError::Segment(
                PcsQueryPlanSegmentError::DuplicateUnitIdentity {
                    unit_index: identity.unit_index,
                    trace_instance_index: identity.trace_instance_index,
                },
            ));
        }
        units.push((identity, unit));
    }
    Ok(units)
}

fn derive_unit_queries(
    seed: &[u8; 32],
    unit_index: u32,
    trace_instance_index: u32,
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
        if trace_instance_index != 0 {
            hasher.update(trace_instance_index.to_le_bytes());
        }
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

fn hash_witness_commitment_segment_for_query_seed(
    hasher: &mut Sha256,
    unit_count: u32,
    segment: &ProofSegment,
) -> Result<(), ProvePcsQueryPlanSegmentError> {
    let identity = witness_commitment_segment_identity(unit_count, segment.id)
        .map_err(|_| ProvePcsQueryPlanSegmentError::LengthOverflow)?
        .ok_or(ProvePcsQueryPlanSegmentError::LengthOverflow)?;
    let unit_index = usize::try_from(identity.unit_index)
        .map_err(|_| ProvePcsQueryPlanSegmentError::LengthOverflow)?;
    let witness = parse_witness_commitment_segment(&segment.data).map_err(|source| {
        ProvePcsQueryPlanSegmentError::InvalidWitnessSegment { unit_index, source }
    })?;
    if witness.unit_index != identity.unit_index {
        return Err(ProvePcsQueryPlanSegmentError::WitnessUnitMismatch {
            segment_unit_index: identity.unit_index,
            payload_unit_index: witness.unit_index,
        });
    }

    hasher.update(b"lzvm-witness-commitment-query-seed-v2");
    hasher.update(segment.id.to_le_bytes());
    hasher.update(identity.unit_index.to_le_bytes());
    hasher.update(identity.trace_instance_index.to_le_bytes());
    hasher.update(witness.unit_index.to_le_bytes());
    hasher.update(witness.trace_rows.to_le_bytes());
    hasher.update(witness.trace_columns.to_le_bytes());
    let stage_count = u64::try_from(witness.stages.len())
        .map_err(|_| ProvePcsQueryPlanSegmentError::LengthOverflow)?;
    hasher.update(stage_count.to_le_bytes());
    for stage in &witness.stages {
        hasher.update(stage.stage_index.to_le_bytes());
        hasher.update(stage.arity.to_le_bytes());
        for word in stage.root {
            hasher.update(word.to_le_bytes());
        }
        hasher.update(stage.tree_digest);
    }
    Ok(())
}

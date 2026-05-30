use lzvm_artifacts::challenge_values_segment::{
    parse_challenge_values_segment, CHALLENGE_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::eth_block_input::EthBlockInput;
use lzvm_artifacts::eth_block_input_segment::{
    encode_eth_block_input_segment, ETH_BLOCK_INPUT_SEGMENT_ID,
};
use lzvm_artifacts::eth_block_public_values::{
    validate_eth_block_public_values, validate_program_image_cache_public_values,
};
use lzvm_artifacts::key_directory::KeyDirectoryCatalog;
use lzvm_artifacts::pcs_evaluation_segment::parse_pcs_evaluation_segment;
use lzvm_artifacts::pcs_nonce_segment::parse_pcs_query_nonce_segment;
use lzvm_artifacts::program_image::ProgramImageCommitmentCache;
use lzvm_artifacts::program_image_segment::{
    encode_program_image_cache_segment, PROGRAM_IMAGE_CACHE_SEGMENT_ID,
};
use lzvm_artifacts::proof::{validate_proof_artifact, ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{public_values_digest, PublicValues};
use lzvm_artifacts::witness_segment::WITNESS_COMMITMENT_SEGMENT_BASE_ID;
use lzvm_field::{Ext3, Felt};

use crate::contribution::{
    build_contribution_segment, build_witness_contribution_input,
    derive_global_challenge_from_proof_segments, derive_worker_contribution_entry,
};
use crate::group_values::build_group_values_segment;
use crate::pcs_transcript::aggregate_pcs_final_query_challenges;
use crate::proof_preflight::{contains_named_eth_block_public_values, public_values_as_fields};
use crate::proof_values::{
    build_pcs_proof_values_segment_from_packed_values, flatten_pcs_proof_values,
    load_pcs_proof_values_from_segments,
};
use crate::setup_preflight::{validate_setup_preflight, validate_setup_preflight_hashes};
use crate::unit_values::{
    build_unit_values_segment_from_packed_values,
    build_unit_values_segment_from_packed_values_batch, ProveUnitValues,
};
use crate::{
    build_constant_opening_segment, build_pcs_evaluation_segment,
    build_pcs_fri_opening_segment_from_transcript_values,
    build_pcs_fri_transcript_values_from_trace_segments, build_pcs_material_manifest_segment,
    build_pcs_query_nonce_segment_with_streams, build_pcs_query_plan_segment,
    build_pcs_query_plan_segment_from_challenge, build_pcs_query_plan_segment_with_bindings,
    build_witness_commitment_segment, build_witness_opening_segment,
    build_witness_opening_segment_batch, ProveExecutionUnitArtifacts, ProvePcsEvaluationValues,
    ProvePcsFriTranscriptTraceSegmentValues, ProveSchedule, ProveWitnessAuxiliaryInputs,
    ProveWitnessCommitments, ProveWitnessTraceCommitments,
};

pub fn build_witness_proof_core_artifact(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    witness_outputs: &[&ProveWitnessCommitments],
) -> Result<ProofArtifact, String> {
    build_witness_proof_core_artifact_with_bindings(
        catalog,
        schedule,
        public_values_hash,
        witness_outputs,
        &[],
    )
}

pub(crate) fn build_witness_proof_core_artifact_with_bindings(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    witness_outputs: &[&ProveWitnessCommitments],
    binding_segments: &[ProofSegment],
) -> Result<ProofArtifact, String> {
    let material_segment = build_pcs_material_manifest_segment(schedule)
        .map_err(|error| format!("build material manifest segment failed: {error}"))?;
    let mut witness_segments = Vec::with_capacity(witness_outputs.len());
    for output in witness_outputs {
        witness_segments.push(
            build_witness_commitment_segment(output)
                .map_err(|error| format!("build witness segment failed: {error}"))?,
        );
    }
    witness_segments.sort_by_key(|segment| segment.id);

    let query_segment = build_pcs_query_plan_segment_with_bindings(
        schedule,
        public_values_hash,
        &material_segment,
        &witness_segments,
        binding_segments,
    )
    .map_err(|error| format!("build query plan segment failed: {error}"))?;
    let constant_opening_segment =
        build_constant_opening_segment(catalog, schedule, &query_segment)
            .map_err(|error| format!("build constant opening segment failed: {error}"))?;
    let opening_segment =
        build_witness_opening_segment_batch(schedule, &query_segment, witness_outputs)
            .map_err(|error| format!("build witness opening segment failed: {error}"))?;

    let mut segments = vec![
        material_segment,
        query_segment,
        constant_opening_segment,
        opening_segment,
    ];
    segments.extend(witness_segments);

    Ok(ProofArtifact {
        setup_hash: schedule.setup_hash,
        public_values_hash,
        segments,
    })
}

pub fn build_witness_proof_artifact(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    witness_outputs: &[&ProveWitnessCommitments],
    proof_values: &[Felt],
    group_values: &[Ext3],
    unit_values: &[ProveUnitValues],
) -> Result<ProofArtifact, String> {
    build_witness_proof_artifact_with_bindings(
        catalog,
        schedule,
        public_values_hash,
        witness_outputs,
        ProofArtifactInputs {
            proof_values,
            group_values,
            unit_values,
            binding_segments: &[],
        },
    )
}

pub fn build_witness_proof_artifact_with_bindings(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    witness_outputs: &[&ProveWitnessCommitments],
    inputs: ProofArtifactInputs<'_>,
) -> Result<ProofArtifact, String> {
    let mut proof = build_witness_proof_core_artifact_with_bindings(
        catalog,
        schedule,
        public_values_hash,
        witness_outputs,
        inputs.binding_segments,
    )?;
    let proof_values_segment = build_pcs_proof_values_segment_from_packed_values(
        &catalog.layout.global_info,
        inputs.proof_values,
    )
    .map_err(|error| format!("build proof values segment failed: {error}"))?;
    if let Some(segment) = proof_values_segment {
        proof.segments.push(segment);
    }
    let group_values_segment =
        build_group_values_segment(&catalog.layout.global_info, inputs.group_values)
            .map_err(|error| format!("build group values segment failed: {error}"))?;
    if let Some(segment) = group_values_segment {
        proof.segments.push(segment);
    }
    let unit_values_segment =
        build_unit_values_segment_from_packed_values_batch(inputs.unit_values)
            .map_err(|error| format!("build unit values segment failed: {error}"))?;
    if let Some(segment) = unit_values_segment {
        proof.segments.push(segment);
    }
    append_binding_segments(&mut proof.segments, inputs.binding_segments.to_vec());
    validate_proof_artifact(&proof)
        .map_err(|error| format!("build proof artifact failed: {error}"))?;
    Ok(proof)
}

pub struct ProofArtifactInputs<'a> {
    pub proof_values: &'a [Felt],
    pub group_values: &'a [Ext3],
    pub unit_values: &'a [ProveUnitValues],
    pub binding_segments: &'a [ProofSegment],
}

pub struct WitnessProofRequest<'a> {
    pub catalog: &'a KeyDirectoryCatalog,
    pub schedule: &'a ProveSchedule,
    pub execution_unit: &'a ProveExecutionUnitArtifacts,
    pub gpu_streams: usize,
    pub public_values: Option<&'a PublicValues>,
    pub unit_values: Option<&'a [Felt]>,
    pub output: &'a ProveWitnessTraceCommitments,
    pub verify_outputs: bool,
    pub program_image_cache: Option<&'a ProgramImageCommitmentCache>,
    pub eth_block_input: Option<&'a EthBlockInput>,
    pub challenge_values_segment: Option<&'a ProofSegment>,
    pub include_contribution_segment: bool,
}

pub fn build_witness_proof_artifact_for_unit(
    request: &WitnessProofRequest<'_>,
) -> Result<Option<ProofArtifact>, String> {
    let Some(public_values) = request.public_values else {
        return Ok(None);
    };
    if public_values.setup_hash != request.schedule.setup_hash {
        return Err("public inputs setup hash mismatch".to_owned());
    }
    let public_values_hash = public_values_digest(public_values)
        .map_err(|error| format!("hash public inputs failed: {error}"))?;
    validate_proof_bindings(
        public_values,
        request.program_image_cache,
        request.eth_block_input,
    )?;
    let binding_segments = build_proof_binding_segments(
        request.program_image_cache,
        request.eth_block_input,
        request.challenge_values_segment,
    )?;
    let binding_segments_slice = binding_segments.as_slice();
    let material_segment = build_pcs_material_manifest_segment(request.schedule)
        .map_err(|error| format!("build material manifest segment failed: {error}"))?;
    let commitments = request.output.commitments();
    let witness_segment = build_witness_commitment_segment(commitments)
        .map_err(|error| format!("build witness segment failed: {error}"))?;
    let transcript_values = if request.output.auxiliary_inputs().evaluations.is_empty() {
        if request.execution_unit.fri_expression_id.is_some() {
            return Err(format!(
                "missing evaluation values for unit {}: expected {}",
                commitments.unit_index(),
                request.execution_unit.expected_evaluation_value_count()
            ));
        }
        None
    } else {
        let evaluation_segment = build_pcs_evaluation_segment(
            request.schedule,
            &[ProvePcsEvaluationValues {
                unit_index: commitments.unit_index(),
                values: request.output.auxiliary_inputs().evaluations.clone(),
            }],
        )
        .map_err(|error| format!("build evaluation segment failed: {error}"))?;
        let values = build_pcs_fri_transcript_values_from_trace_segments(
            request.schedule,
            &[ProvePcsFriTranscriptTraceSegmentValues {
                unit_index: commitments.unit_index(),
                execution_unit: request.execution_unit,
                trace: request.output.trace(),
                publics: request.output.publics(),
                auxiliary_inputs: request.output.auxiliary_inputs(),
                material_segment: &material_segment,
                witness_segment: &witness_segment,
                evaluation_segment: &evaluation_segment,
                binding_segments: binding_segments_slice,
            }],
        )
        .map_err(|error| format!("build FRI transcript values failed: {error}"))?;
        Some((evaluation_segment, values))
    };
    let query_segment = match &transcript_values {
        Some((_, values)) => {
            let final_query_challenges = values
                .iter()
                .map(|value| value.commitments.final_query_challenge)
                .collect::<Vec<_>>();
            let final_query_challenge =
                aggregate_pcs_final_query_challenges(&final_query_challenges)
                    .map_err(|error| format!("build query challenge failed: {error}"))?;
            let nonce_segment = build_pcs_query_nonce_segment_with_streams(
                request.schedule,
                final_query_challenge,
                request.gpu_streams,
            )
            .map_err(|error| format!("build query nonce segment failed: {error}"))?;
            let nonce = Felt::from_u64(
                parse_pcs_query_nonce_segment(&nonce_segment.data)
                    .map_err(|error| format!("parse query nonce segment failed: {error}"))?
                    .nonce,
            );
            let query_segment = build_pcs_query_plan_segment_from_challenge(
                request.schedule,
                std::slice::from_ref(&witness_segment),
                final_query_challenge,
                nonce,
            )
            .map_err(|error| format!("build query plan segment failed: {error}"))?;
            (query_segment, Some(nonce_segment))
        }
        None => {
            let query_segment = if binding_segments_slice.is_empty() {
                build_pcs_query_plan_segment(
                    request.schedule,
                    public_values_hash,
                    &material_segment,
                    std::slice::from_ref(&witness_segment),
                )
                .map_err(|error| format!("build query plan segment failed: {error}"))?
            } else {
                build_pcs_query_plan_segment_with_bindings(
                    request.schedule,
                    public_values_hash,
                    &material_segment,
                    std::slice::from_ref(&witness_segment),
                    binding_segments_slice,
                )
                .map_err(|error| format!("build query plan segment failed: {error}"))?
            };
            (query_segment, None)
        }
    };
    let (query_segment, nonce_segment) = query_segment;
    let constant_opening_segment =
        build_constant_opening_segment(request.catalog, request.schedule, &query_segment)
            .map_err(|error| format!("build constant opening segment failed: {error}"))?;
    let opening_segment =
        build_witness_opening_segment(request.schedule, &query_segment, commitments)
            .map_err(|error| format!("build witness opening segment failed: {error}"))?;
    let unit_index = commitments.unit_index();
    let unit = request
        .schedule
        .units
        .get(unit_index)
        .ok_or_else(|| format!("unit values segment unit index out of range: {unit_index}"))?;
    let unit_values = request
        .unit_values
        .unwrap_or(&request.output.auxiliary_inputs().unit_values);
    let unit_values_segment =
        build_unit_values_segment_from_packed_values(unit_index, &unit.unit_value_map, unit_values)
            .map_err(|error| format!("build unit values segment failed: {error}"))?;
    let proof_values_segment = build_pcs_proof_values_segment_from_packed_values(
        &request.catalog.layout.global_info,
        &request.output.auxiliary_inputs().proof_values,
    )
    .map_err(|error| format!("build proof values segment failed: {error}"))?;
    let group_values_segment = build_group_values_segment(
        &request.catalog.layout.global_info,
        &request.output.auxiliary_inputs().group_values,
    )
    .map_err(|error| format!("build group values segment failed: {error}"))?;
    let mut segments = vec![
        material_segment,
        query_segment,
        constant_opening_segment,
        opening_segment,
        witness_segment,
    ];
    if let Some((evaluation_segment, transcript_values)) = transcript_values {
        let fri_segment = build_pcs_fri_opening_segment_from_transcript_values(
            request.schedule,
            &segments[1],
            &transcript_values,
        )
        .map_err(|error| format!("build FRI opening segment failed: {error}"))?;
        segments.push(evaluation_segment);
        segments.push(fri_segment);
    }
    if let Some(nonce_segment) = nonce_segment {
        segments.push(nonce_segment);
    }
    if let Some(proof_values_segment) = proof_values_segment {
        segments.push(proof_values_segment);
    }
    if let Some(group_values_segment) = group_values_segment {
        segments.push(group_values_segment);
    }
    if let Some(unit_values_segment) = unit_values_segment {
        segments.push(unit_values_segment);
    }
    let has_contribution_segment = if request.include_contribution_segment {
        let contribution_source = WitnessContributionSource {
            output: request.output,
            packed_unit_values: unit_values,
        };
        if let Some(contribution_segment) = build_witness_contribution_segment(
            request.catalog,
            request.schedule,
            std::slice::from_ref(&contribution_source),
        )? {
            segments.push(contribution_segment);
            true
        } else {
            false
        }
    } else {
        false
    };
    append_binding_segments(&mut segments, binding_segments);
    let proof = ProofArtifact {
        setup_hash: request.schedule.setup_hash,
        public_values_hash,
        segments,
    };
    if has_contribution_segment {
        if request.challenge_values_segment.is_none() {
            return Err(
                "verify contribution proof output failed: missing challenge values segment"
                    .to_owned(),
            );
        }
        validate_contribution_proof_output(request.catalog, &proof, public_values)?;
    }
    if request.verify_outputs {
        validate_setup_preflight(request.catalog, &proof, public_values)
            .map_err(|error| format!("verify proof output failed: {error}"))?;
    }
    Ok(Some(proof))
}

pub fn build_witness_contribution_proof_artifact_for_unit(
    request: &WitnessProofRequest<'_>,
) -> Result<Option<ProofArtifact>, String> {
    let Some(public_values) = request.public_values else {
        return Ok(None);
    };
    if public_values.setup_hash != request.schedule.setup_hash {
        return Err("public inputs setup hash mismatch".to_owned());
    }
    let public_values_hash = public_values_digest(public_values)
        .map_err(|error| format!("hash public inputs failed: {error}"))?;
    validate_proof_bindings(
        public_values,
        request.program_image_cache,
        request.eth_block_input,
    )?;
    let binding_segments = build_proof_binding_segments(
        request.program_image_cache,
        request.eth_block_input,
        request.challenge_values_segment,
    )?;
    let unit_values = request
        .unit_values
        .unwrap_or(&request.output.auxiliary_inputs().unit_values);
    let contribution_source = WitnessContributionSource {
        output: request.output,
        packed_unit_values: unit_values,
    };
    validate_witness_contribution_sources(
        request.schedule,
        std::slice::from_ref(&contribution_source),
    )?;
    let contribution_segment = build_witness_contribution_segment(
        request.catalog,
        request.schedule,
        std::slice::from_ref(&contribution_source),
    )?
    .ok_or_else(|| "contribution proof has no contribution segment".to_owned())?;
    let proof_values_segment = build_pcs_proof_values_segment_from_packed_values(
        &request.catalog.layout.global_info,
        &request.output.auxiliary_inputs().proof_values,
    )
    .map_err(|error| format!("build proof values segment failed: {error}"))?;

    let mut segments = Vec::new();
    if let Some(proof_values_segment) = proof_values_segment {
        segments.push(proof_values_segment);
    }
    segments.push(contribution_segment);
    append_binding_segments(&mut segments, binding_segments);
    let proof = ProofArtifact {
        setup_hash: request.schedule.setup_hash,
        public_values_hash,
        segments,
    };
    if request.verify_outputs || request.challenge_values_segment.is_some() {
        validate_contribution_proof_output(request.catalog, &proof, public_values)?;
    }
    Ok(Some(proof))
}

pub struct WitnessAllUnitsProofRequest<'a> {
    pub catalog: &'a KeyDirectoryCatalog,
    pub schedule: &'a ProveSchedule,
    pub execution_units: &'a [ProveExecutionUnitArtifacts],
    pub gpu_streams: usize,
    pub public_values: Option<&'a PublicValues>,
    pub outputs: &'a [ProveWitnessTraceCommitments],
    pub auxiliary_inputs: &'a ProveWitnessAuxiliaryInputs,
    pub unit_values: &'a [ProveUnitValues],
    pub evaluation_values_segment: Option<&'a ProofSegment>,
    pub verify_outputs: bool,
    pub program_image_cache: Option<&'a ProgramImageCommitmentCache>,
    pub eth_block_input: Option<&'a EthBlockInput>,
    pub challenge_values_segment: Option<&'a ProofSegment>,
    pub include_contribution_segment: bool,
}

pub fn build_witness_contribution_proof_artifact_for_all_units(
    request: &WitnessAllUnitsProofRequest<'_>,
) -> Result<Option<ProofArtifact>, String> {
    let Some(public_values) = request.public_values else {
        return Ok(None);
    };
    if public_values.setup_hash != request.schedule.setup_hash {
        return Err("public inputs setup hash mismatch".to_owned());
    }
    let public_values_hash = public_values_digest(public_values)
        .map_err(|error| format!("hash public inputs failed: {error}"))?;
    validate_proof_bindings(
        public_values,
        request.program_image_cache,
        request.eth_block_input,
    )?;
    let binding_segments = build_proof_binding_segments(
        request.program_image_cache,
        request.eth_block_input,
        request.challenge_values_segment,
    )?;
    let contribution_sources = request
        .outputs
        .iter()
        .map(|output| {
            let unit_index = output.commitments().unit_index();
            let packed_unit_values = request
                .unit_values
                .iter()
                .find(|values| values.unit_index == unit_index)
                .map(|values| values.packed_values.as_slice())
                .unwrap_or_else(|| output.auxiliary_inputs().unit_values.as_slice());
            WitnessContributionSource {
                output,
                packed_unit_values,
            }
        })
        .collect::<Vec<_>>();
    validate_witness_contribution_sources(request.schedule, &contribution_sources)?;
    let contribution_segment = build_witness_contribution_segment(
        request.catalog,
        request.schedule,
        &contribution_sources,
    )?
    .ok_or_else(|| "contribution proof has no contribution segment".to_owned())?;
    let proof_values =
        collect_global_proof_values(request.outputs, &request.auxiliary_inputs.proof_values)?;
    let proof_values_segment = build_pcs_proof_values_segment_from_packed_values(
        &request.catalog.layout.global_info,
        &proof_values,
    )
    .map_err(|error| format!("build proof values segment failed: {error}"))?;

    let mut segments = Vec::new();
    if let Some(proof_values_segment) = proof_values_segment {
        segments.push(proof_values_segment);
    }
    segments.push(contribution_segment);
    append_binding_segments(&mut segments, binding_segments);
    let proof = ProofArtifact {
        setup_hash: request.schedule.setup_hash,
        public_values_hash,
        segments,
    };
    if request.verify_outputs || request.challenge_values_segment.is_some() {
        validate_contribution_proof_output(request.catalog, &proof, public_values)?;
    }
    Ok(Some(proof))
}

pub fn build_witness_proof_artifact_for_all_units(
    request: &WitnessAllUnitsProofRequest<'_>,
) -> Result<Option<ProofArtifact>, String> {
    let Some(public_values) = request.public_values else {
        return Ok(None);
    };
    if public_values.setup_hash != request.schedule.setup_hash {
        return Err("public inputs setup hash mismatch".to_owned());
    }
    let public_values_hash = public_values_digest(public_values)
        .map_err(|error| format!("hash public inputs failed: {error}"))?;
    validate_proof_bindings(
        public_values,
        request.program_image_cache,
        request.eth_block_input,
    )?;
    let binding_segments = build_proof_binding_segments(
        request.program_image_cache,
        request.eth_block_input,
        request.challenge_values_segment,
    )?;
    let binding_segments_slice = binding_segments.as_slice();
    let proof_values =
        collect_global_proof_values(request.outputs, &request.auxiliary_inputs.proof_values)?;
    let group_values =
        collect_global_group_values(request.outputs, &request.auxiliary_inputs.group_values)?;
    let evaluation_values =
        collect_all_units_evaluation_values(request.outputs, &request.auxiliary_inputs.evaluations);
    let proof_unit_values =
        collect_proof_unit_values(request.schedule, request.outputs, request.unit_values)?;
    let witness_outputs = request
        .outputs
        .iter()
        .map(|output| output.commitments())
        .collect::<Vec<_>>();
    let needs_transcript = all_units_transcript_required(
        request.execution_units,
        request.outputs,
        &evaluation_values,
        request.evaluation_values_segment.is_some(),
    )?;
    let mut proof = if needs_transcript {
        build_witness_transcript_proof_artifact_for_all_units(
            request,
            public_values_hash,
            &witness_outputs,
            AllUnitsTranscriptProofInputs {
                binding_segments: binding_segments_slice,
                proof_values: &proof_values,
                group_values: &group_values,
                evaluation_values: &evaluation_values,
                unit_values: &proof_unit_values,
            },
        )?
    } else if binding_segments_slice.is_empty() {
        build_witness_proof_artifact(
            request.catalog,
            request.schedule,
            public_values_hash,
            &witness_outputs,
            &proof_values,
            &group_values,
            &proof_unit_values,
        )?
    } else {
        build_witness_proof_artifact_with_bindings(
            request.catalog,
            request.schedule,
            public_values_hash,
            &witness_outputs,
            ProofArtifactInputs {
                proof_values: &proof_values,
                group_values: &group_values,
                unit_values: &proof_unit_values,
                binding_segments: binding_segments_slice,
            },
        )?
    };
    let has_contribution_segment = if request.include_contribution_segment {
        let contribution_sources = request
            .outputs
            .iter()
            .map(|output| {
                let unit_index = output.commitments().unit_index();
                let packed_unit_values = proof_unit_values
                    .iter()
                    .find(|values| values.unit_index == unit_index)
                    .map(|values| values.packed_values.as_slice())
                    .unwrap_or_else(|| output.auxiliary_inputs().unit_values.as_slice());
                WitnessContributionSource {
                    output,
                    packed_unit_values,
                }
            })
            .collect::<Vec<_>>();
        if let Some(contribution_segment) = build_witness_contribution_segment(
            request.catalog,
            request.schedule,
            &contribution_sources,
        )? {
            proof.segments.push(contribution_segment);
            true
        } else {
            false
        }
    } else {
        false
    };
    if needs_transcript {
        append_binding_segments(&mut proof.segments, binding_segments);
    }
    if has_contribution_segment {
        if request.challenge_values_segment.is_none() {
            return Err(
                "verify contribution proof output failed: missing challenge values segment"
                    .to_owned(),
            );
        }
        validate_contribution_proof_output(request.catalog, &proof, public_values)?;
    }
    if request.verify_outputs {
        validate_setup_preflight(request.catalog, &proof, public_values)
            .map_err(|error| format!("verify proof output failed: {error}"))?;
    }
    Ok(Some(proof))
}

fn build_program_image_cache_proof_segment(
    cache: Option<&ProgramImageCommitmentCache>,
) -> Result<Option<ProofSegment>, String> {
    let Some(cache) = cache else {
        return Ok(None);
    };
    validate_program_image_cache_tree_root(cache)?;
    let data = encode_program_image_cache_segment(cache)
        .map_err(|error| format!("build program image cache segment failed: {error}"))?;
    Ok(Some(ProofSegment {
        id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
        data,
    }))
}

fn validate_program_image_cache_tree_root(
    cache: &ProgramImageCommitmentCache,
) -> Result<(), String> {
    for (word_index, word) in cache.tree_root.iter().copied().enumerate() {
        Felt::from_canonical(word).map_err(|source| {
            format!("program image cache tree root word {word_index} is non-canonical: {source}")
        })?;
    }
    Ok(())
}

fn build_eth_block_input_proof_segment(
    input: Option<&EthBlockInput>,
) -> Result<Option<ProofSegment>, String> {
    let Some(input) = input else {
        return Ok(None);
    };
    let data = encode_eth_block_input_segment(input)
        .map_err(|error| format!("build ETH block input segment failed: {error}"))?;
    Ok(Some(ProofSegment {
        id: ETH_BLOCK_INPUT_SEGMENT_ID,
        data,
    }))
}

fn validate_proof_bindings(
    public_values: &PublicValues,
    program_image_cache: Option<&ProgramImageCommitmentCache>,
    eth_block_input: Option<&EthBlockInput>,
) -> Result<(), String> {
    validate_program_image_cache_binding(public_values, program_image_cache)?;
    validate_eth_block_binding(public_values, eth_block_input)
}

fn validate_program_image_cache_binding(
    public_values: &PublicValues,
    cache: Option<&ProgramImageCommitmentCache>,
) -> Result<(), String> {
    if let Some(cache) = cache {
        if cache.constraint_system_digest != public_values.setup_hash {
            return Err("program image cache setup hash mismatch".to_owned());
        }
    }
    validate_program_image_cache_public_values(public_values, cache)
        .map_err(|error| error.to_string())
}

fn validate_eth_block_binding(
    public_values: &PublicValues,
    input: Option<&EthBlockInput>,
) -> Result<(), String> {
    if let Some(input) = input {
        validate_eth_block_public_values(input, public_values)
            .map_err(|error| error.to_string())?;
    } else if contains_named_eth_block_public_values(public_values) {
        return Err("missing ETH block input proof segment".to_owned());
    }
    Ok(())
}

fn validate_contribution_proof_output(
    catalog: &KeyDirectoryCatalog,
    proof: &ProofArtifact,
    public_values: &PublicValues,
) -> Result<(), String> {
    validate_setup_preflight_hashes(catalog, proof, public_values)
        .map_err(|error| format!("verify contribution proof output failed: {error}"))?;
    if proof
        .segments
        .iter()
        .any(|segment| segment.id == CHALLENGE_VALUES_SEGMENT_ID)
    {
        validate_contribution_proof_challenge_values(catalog, proof, public_values)?;
    }
    Ok(())
}

fn validate_contribution_proof_challenge_values(
    catalog: &KeyDirectoryCatalog,
    proof: &ProofArtifact,
    public_values: &PublicValues,
) -> Result<(), String> {
    let segment = proof
        .segments
        .iter()
        .find(|segment| segment.id == CHALLENGE_VALUES_SEGMENT_ID)
        .ok_or_else(|| {
            "verify contribution proof output failed: missing challenge values segment".to_owned()
        })?;
    let challenge_values = parse_challenge_values_segment(&segment.data)
        .map_err(|error| format!("verify contribution proof output failed: {error}"))?;
    let public_fields = public_values_as_fields(public_values)
        .map_err(|error| format!("verify contribution proof output failed: {error}"))?;
    let proof_values =
        load_pcs_proof_values_from_segments(&catalog.layout.global_info, &proof.segments)
            .map_err(|error| format!("verify contribution proof output failed: {error}"))?;
    let packed_proof_values = flatten_pcs_proof_values(&catalog.layout.global_info, &proof_values)
        .map_err(|error| format!("verify contribution proof output failed: {error}"))?;
    let expected = derive_global_challenge_from_proof_segments(
        &catalog.layout.global_info,
        &public_fields,
        &packed_proof_values,
        &proof.segments,
    )
    .map_err(|error| format!("verify contribution proof output failed: {error}"))?;
    if challenge_values.values.as_slice() != [expected.to_u64s()] {
        return Err(
            "verify contribution proof output failed: contribution challenge values mismatch"
                .to_owned(),
        );
    }
    Ok(())
}

fn build_proof_binding_segments(
    cache: Option<&ProgramImageCommitmentCache>,
    eth_block_input: Option<&EthBlockInput>,
    challenge_values_segment: Option<&ProofSegment>,
) -> Result<Vec<ProofSegment>, String> {
    let mut segments = Vec::new();
    if let Some(segment) = build_program_image_cache_proof_segment(cache)? {
        segments.push(segment);
    }
    if let Some(segment) = build_eth_block_input_proof_segment(eth_block_input)? {
        segments.push(segment);
    }
    if let Some(segment) = challenge_values_segment {
        if segment.id != CHALLENGE_VALUES_SEGMENT_ID {
            return Err(format!(
                "challenge values proof segment id mismatch: expected {CHALLENGE_VALUES_SEGMENT_ID}, found {}",
                segment.id
            ));
        }
        parse_challenge_values_segment(&segment.data)
            .map_err(|error| format!("invalid challenge values proof segment: {error}"))?;
        segments.push(segment.clone());
    }
    Ok(segments)
}

fn append_binding_segments(segments: &mut Vec<ProofSegment>, binding_segments: Vec<ProofSegment>) {
    segments.extend(binding_segments);
}

struct WitnessContributionSource<'a> {
    output: &'a ProveWitnessTraceCommitments,
    packed_unit_values: &'a [Felt],
}

fn build_witness_contribution_segment(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    sources: &[WitnessContributionSource<'_>],
) -> Result<Option<ProofSegment>, String> {
    if sources.is_empty() || catalog.layout.global_info.lattice_size.is_none() {
        return Ok(None);
    }

    let mut entries = Vec::with_capacity(sources.len());
    for source in sources {
        let output = source.output;
        let unit_index = output.commitments().unit_index();
        let unit = schedule
            .units
            .get(unit_index)
            .ok_or_else(|| format!("witness contribution unit index out of range: {unit_index}"))?;
        if source.packed_unit_values.is_empty() && !unit.unit_value_map.is_empty() {
            continue;
        }
        let catalog_unit = catalog.units.get(unit_index).ok_or_else(|| {
            format!("witness contribution catalog unit index out of range: {unit_index}")
        })?;
        let input = build_witness_contribution_input(
            &catalog_unit.verification_key,
            unit,
            output,
            source.packed_unit_values,
        )
        .map_err(|error| {
            format!("build witness contribution input failed for unit {unit_index}: {error}")
        })?;
        let worker_index = u32::try_from(unit_index).map_err(|_| {
            format!("witness contribution unit index does not fit u32: {unit_index}")
        })?;
        let group_id = unit.group_id.unwrap_or(0);
        let group_id = u32::try_from(group_id)
            .map_err(|_| format!("witness contribution group id does not fit u32: {group_id}"))?;
        let entry = derive_worker_contribution_entry(
            &catalog.layout.global_info,
            worker_index,
            group_id,
            &[input],
        )
        .map_err(|error| {
            format!("build witness contribution entry failed for unit {unit_index}: {error}")
        })?;
        entries.push(entry);
    }

    build_contribution_segment(&entries)
        .map_err(|error| format!("build contribution segment failed: {error}"))
}

fn validate_witness_contribution_sources(
    schedule: &ProveSchedule,
    sources: &[WitnessContributionSource<'_>],
) -> Result<(), String> {
    for source in sources {
        let unit_index = source.output.commitments().unit_index();
        let unit = schedule
            .units
            .get(unit_index)
            .ok_or_else(|| format!("witness contribution unit index out of range: {unit_index}"))?;
        if source.packed_unit_values.is_empty() && !unit.unit_value_map.is_empty() {
            return Err(format!(
                "missing unit values for contribution unit {unit_index}: expected {}",
                unit.unit_value_map.len()
            ));
        }
    }
    Ok(())
}

fn all_units_transcript_required(
    execution_units: &[ProveExecutionUnitArtifacts],
    outputs: &[ProveWitnessTraceCommitments],
    evaluation_values: &[ProvePcsEvaluationValues],
    has_evaluation_segment: bool,
) -> Result<bool, String> {
    let mut has_fri_unit = false;
    for output in outputs {
        let unit_index = output.commitments().unit_index();
        let execution_unit = execution_units
            .get(unit_index)
            .ok_or_else(|| format!("output unit index out of range: {unit_index}"))?;
        if execution_unit.fri_expression_id.is_some() {
            has_fri_unit = true;
            let has_unit_evaluations = evaluation_values
                .iter()
                .any(|values| values.unit_index == unit_index && !values.values.is_empty());
            if !has_unit_evaluations && !has_evaluation_segment {
                return Err(format!(
                    "missing evaluation values for unit {unit_index}: expected {}",
                    execution_unit.expected_evaluation_value_count()
                ));
            }
        }
    }
    Ok(has_fri_unit || !evaluation_values.is_empty() || has_evaluation_segment)
}

fn build_witness_transcript_proof_artifact_for_all_units(
    request: &WitnessAllUnitsProofRequest<'_>,
    public_values_hash: [u8; 32],
    witness_outputs: &[&ProveWitnessCommitments],
    proof_inputs: AllUnitsTranscriptProofInputs<'_>,
) -> Result<ProofArtifact, String> {
    let material_segment = build_pcs_material_manifest_segment(request.schedule)
        .map_err(|error| format!("build material manifest segment failed: {error}"))?;
    let mut witness_segments = Vec::with_capacity(witness_outputs.len());
    for output in witness_outputs {
        witness_segments.push(
            build_witness_commitment_segment(output)
                .map_err(|error| format!("build witness segment failed: {error}"))?,
        );
    }
    witness_segments.sort_by_key(|segment| segment.id);

    let (evaluation_segment, transcript_evaluation_values) = match request.evaluation_values_segment
    {
        Some(segment) => (
            segment.clone(),
            collect_evaluation_values_from_segment(segment)?,
        ),
        None => (
            build_pcs_evaluation_segment(request.schedule, proof_inputs.evaluation_values)
                .map_err(|error| format!("build evaluation segment failed: {error}"))?,
            proof_inputs.evaluation_values.to_vec(),
        ),
    };
    let transcript_auxiliary_inputs = request
        .outputs
        .iter()
        .map(|output| {
            let unit_index = output.commitments().unit_index();
            let mut auxiliary_inputs = output.auxiliary_inputs().clone();
            if let Some(values) = proof_inputs
                .unit_values
                .iter()
                .find(|values| values.unit_index == unit_index)
            {
                auxiliary_inputs.unit_values = values.packed_values.clone();
            }
            if !proof_inputs.proof_values.is_empty() {
                auxiliary_inputs.proof_values = proof_inputs.proof_values.to_vec();
            }
            if !proof_inputs.group_values.is_empty() {
                auxiliary_inputs.group_values = proof_inputs.group_values.to_vec();
            }
            if let Some(values) = transcript_evaluation_values
                .iter()
                .find(|values| values.unit_index == unit_index)
            {
                auxiliary_inputs.evaluations = values.values.clone();
            }
            auxiliary_inputs
        })
        .collect::<Vec<_>>();
    let transcript_inputs = request
        .outputs
        .iter()
        .zip(transcript_auxiliary_inputs.iter())
        .map(|(output, auxiliary_inputs)| {
            let commitments = output.commitments();
            let unit_index = commitments.unit_index();
            let execution_unit = request
                .execution_units
                .get(unit_index)
                .ok_or_else(|| format!("output unit index out of range: {unit_index}"))?;
            let unit_index_u32 = u32::try_from(unit_index).map_err(|_| {
                format!("witness segment unit index does not fit u32: {unit_index}")
            })?;
            let expected_segment_id = WITNESS_COMMITMENT_SEGMENT_BASE_ID
                .checked_add(unit_index_u32)
                .ok_or_else(|| format!("witness segment unit index overflow: {unit_index}"))?;
            let witness_segment = witness_segments
                .iter()
                .find(|segment| segment.id == expected_segment_id)
                .ok_or_else(|| format!("missing witness segment for unit {unit_index}"))?;
            Ok(ProvePcsFriTranscriptTraceSegmentValues {
                unit_index,
                execution_unit,
                trace: output.trace(),
                publics: output.publics(),
                auxiliary_inputs,
                material_segment: &material_segment,
                witness_segment,
                evaluation_segment: &evaluation_segment,
                binding_segments: proof_inputs.binding_segments,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let transcript_values =
        build_pcs_fri_transcript_values_from_trace_segments(request.schedule, &transcript_inputs)
            .map_err(|error| format!("build FRI transcript values failed: {error}"))?;
    let final_query_challenges = transcript_values
        .iter()
        .map(|value| value.commitments.final_query_challenge)
        .collect::<Vec<_>>();
    let final_query_challenge = aggregate_pcs_final_query_challenges(&final_query_challenges)
        .map_err(|error| format!("build query challenge failed: {error}"))?;
    let nonce_segment = build_pcs_query_nonce_segment_with_streams(
        request.schedule,
        final_query_challenge,
        request.gpu_streams,
    )
    .map_err(|error| format!("build query nonce segment failed: {error}"))?;
    let nonce = Felt::from_u64(
        parse_pcs_query_nonce_segment(&nonce_segment.data)
            .map_err(|error| format!("parse query nonce segment failed: {error}"))?
            .nonce,
    );
    let query_segment = build_pcs_query_plan_segment_from_challenge(
        request.schedule,
        &witness_segments,
        final_query_challenge,
        nonce,
    )
    .map_err(|error| format!("build query plan segment failed: {error}"))?;
    let constant_opening_segment =
        build_constant_opening_segment(request.catalog, request.schedule, &query_segment)
            .map_err(|error| format!("build constant opening segment failed: {error}"))?;
    let opening_segment =
        build_witness_opening_segment_batch(request.schedule, &query_segment, witness_outputs)
            .map_err(|error| format!("build witness opening segment failed: {error}"))?;
    let fri_segment = build_pcs_fri_opening_segment_from_transcript_values(
        request.schedule,
        &query_segment,
        &transcript_values,
    )
    .map_err(|error| format!("build FRI opening segment failed: {error}"))?;

    let proof_values_segment = build_pcs_proof_values_segment_from_packed_values(
        &request.catalog.layout.global_info,
        proof_inputs.proof_values,
    )
    .map_err(|error| format!("build proof values segment failed: {error}"))?;
    let group_values_segment = build_group_values_segment(
        &request.catalog.layout.global_info,
        proof_inputs.group_values,
    )
    .map_err(|error| format!("build group values segment failed: {error}"))?;
    let unit_values_segment =
        build_unit_values_segment_from_packed_values_batch(proof_inputs.unit_values)
            .map_err(|error| format!("build unit values segment failed: {error}"))?;

    let mut segments = vec![
        material_segment,
        query_segment,
        constant_opening_segment,
        opening_segment,
    ];
    segments.extend(witness_segments);
    segments.push(evaluation_segment);
    segments.push(fri_segment);
    segments.push(nonce_segment);
    if let Some(segment) = proof_values_segment {
        segments.push(segment);
    }
    if let Some(segment) = group_values_segment {
        segments.push(segment);
    }
    if let Some(segment) = unit_values_segment {
        segments.push(segment);
    }

    Ok(ProofArtifact {
        setup_hash: request.schedule.setup_hash,
        public_values_hash,
        segments,
    })
}

struct AllUnitsTranscriptProofInputs<'a> {
    binding_segments: &'a [ProofSegment],
    proof_values: &'a [Felt],
    group_values: &'a [Ext3],
    evaluation_values: &'a [ProvePcsEvaluationValues],
    unit_values: &'a [ProveUnitValues],
}

fn collect_evaluation_values_from_segment(
    segment: &ProofSegment,
) -> Result<Vec<ProvePcsEvaluationValues>, String> {
    let parsed = parse_pcs_evaluation_segment(&segment.data)
        .map_err(|error| format!("parse evaluation segment failed: {error}"))?;
    parsed
        .units
        .into_iter()
        .map(|unit| {
            let unit_index = usize::try_from(unit.unit_index).map_err(|_| {
                format!(
                    "evaluation segment unit index does not fit usize: {}",
                    unit.unit_index
                )
            })?;
            let values = unit
                .values
                .into_iter()
                .enumerate()
                .map(|(index, words)| {
                    let c0 = Felt::from_canonical(words[0]).map_err(|error| {
                        format!(
                            "invalid evaluation segment unit {unit_index} value {index}: {error}"
                        )
                    })?;
                    let c1 = Felt::from_canonical(words[1]).map_err(|error| {
                        format!(
                            "invalid evaluation segment unit {unit_index} value {index}: {error}"
                        )
                    })?;
                    let c2 = Felt::from_canonical(words[2]).map_err(|error| {
                        format!(
                            "invalid evaluation segment unit {unit_index} value {index}: {error}"
                        )
                    })?;
                    Ok(Ext3::new(c0, c1, c2))
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(ProvePcsEvaluationValues { unit_index, values })
        })
        .collect()
}

fn collect_global_proof_values(
    outputs: &[ProveWitnessTraceCommitments],
    explicit_values: &[Felt],
) -> Result<Vec<Felt>, String> {
    if !explicit_values.is_empty() {
        return Ok(explicit_values.to_vec());
    }

    let mut values: Option<&[Felt]> = None;
    for output in outputs {
        let candidate = output.auxiliary_inputs().proof_values.as_slice();
        if candidate.is_empty() {
            continue;
        }
        if let Some(existing) = values {
            if existing != candidate {
                let unit_index = output.commitments().unit_index();
                return Err(format!(
                    "conflicting proof values across witness outputs at unit {unit_index}"
                ));
            }
        } else {
            values = Some(candidate);
        }
    }

    Ok(values.map_or_else(Vec::new, ToOwned::to_owned))
}

fn collect_global_group_values(
    outputs: &[ProveWitnessTraceCommitments],
    explicit_values: &[Ext3],
) -> Result<Vec<Ext3>, String> {
    if !explicit_values.is_empty() {
        return Ok(explicit_values.to_vec());
    }

    let mut values: Option<&[Ext3]> = None;
    for output in outputs {
        let candidate = output.auxiliary_inputs().group_values.as_slice();
        if candidate.is_empty() {
            continue;
        }
        if let Some(existing) = values {
            if existing != candidate {
                let unit_index = output.commitments().unit_index();
                return Err(format!(
                    "conflicting group values across witness outputs at unit {unit_index}"
                ));
            }
        } else {
            values = Some(candidate);
        }
    }

    Ok(values.map_or_else(Vec::new, ToOwned::to_owned))
}

fn collect_all_units_evaluation_values(
    outputs: &[ProveWitnessTraceCommitments],
    explicit_values: &[Ext3],
) -> Vec<ProvePcsEvaluationValues> {
    if !explicit_values.is_empty() {
        return outputs
            .iter()
            .map(|output| ProvePcsEvaluationValues {
                unit_index: output.commitments().unit_index(),
                values: explicit_values.to_vec(),
            })
            .collect();
    }

    outputs
        .iter()
        .filter_map(|output| {
            let values = output.auxiliary_inputs().evaluations.clone();
            if values.is_empty() {
                None
            } else {
                Some(ProvePcsEvaluationValues {
                    unit_index: output.commitments().unit_index(),
                    values,
                })
            }
        })
        .collect()
}

fn collect_proof_unit_values(
    schedule: &ProveSchedule,
    outputs: &[ProveWitnessTraceCommitments],
    explicit_values: &[ProveUnitValues],
) -> Result<Vec<ProveUnitValues>, String> {
    if !explicit_values.is_empty() {
        return Ok(explicit_values.to_vec());
    }

    let mut values = Vec::with_capacity(outputs.len());
    for output in outputs {
        let unit_index = output.commitments().unit_index();
        let unit = schedule
            .units
            .get(unit_index)
            .ok_or_else(|| format!("unit values segment unit index out of range: {unit_index}"))?;
        let packed_values = output.auxiliary_inputs().unit_values.clone();
        if !packed_values.is_empty() || !unit.unit_value_map.is_empty() {
            values.push(ProveUnitValues {
                unit_index,
                unit_value_map: unit.unit_value_map.clone(),
                packed_values,
            });
        }
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lzvm_artifacts::eth_block_input::build_eth_block_input;
    use lzvm_artifacts::eth_block_public_values::public_values_from_eth_block_input;
    use lzvm_artifacts::program_image::ProgramImageGpuMode;
    use lzvm_artifacts::public_values::PublicValueEntry;
    use lzvm_field::MODULUS;

    #[test]
    fn rejects_eth_block_public_values_without_bound_input() {
        let block_input =
            build_eth_block_input(&sample_block_rlp()).expect("block input should build");
        let public_values = public_values_from_eth_block_input([0x44; 32], &block_input);

        let error = validate_eth_block_binding(&public_values, None)
            .expect_err("ETH block public values should require a bound input");

        assert_eq!(error, "missing ETH block input proof segment");
    }

    #[test]
    fn rejects_program_image_cache_setup_hash_mismatches_for_binding() {
        let public_values = PublicValues {
            schema_version: 1,
            setup_hash: [0x44; 32],
            values: vec![PublicValueEntry {
                name: "sample_public".to_owned(),
                elements: vec![19],
            }],
        };
        let cache = ProgramImageCommitmentCache {
            program_digest: [0x11; 32],
            source_image_digest: [0x22; 32],
            constraint_system_digest: [0x99; 32],
            tree_root: [1, 2, 3, 4],
            trace_row_count: 1024,
            trace_column_count: 17,
            blowup_factor: 8,
            merkle_tree_arity: 4,
            gpu_mode: ProgramImageGpuMode::Cuda,
        };

        let error = validate_program_image_cache_binding(&public_values, Some(&cache))
            .expect_err("program image cache should match public input setup hash");

        assert_eq!(error, "program image cache setup hash mismatch");
    }

    #[test]
    fn rejects_invalid_challenge_values_binding_segments() {
        let segment = ProofSegment {
            id: CHALLENGE_VALUES_SEGMENT_ID,
            data: vec![1],
        };

        let error = build_proof_binding_segments(None, None, Some(&segment))
            .expect_err("challenge values binding segment should parse");

        assert_eq!(
            error,
            "invalid challenge values proof segment: truncated challenge values segment: needed 4, available 1"
        );
    }

    #[test]
    fn rejects_non_canonical_program_image_cache_tree_root_for_binding_segment() {
        let cache = ProgramImageCommitmentCache {
            program_digest: [0x11; 32],
            source_image_digest: [0x22; 32],
            constraint_system_digest: [0x33; 32],
            tree_root: [MODULUS, 11, 12, 13],
            trace_row_count: 1024,
            trace_column_count: 17,
            blowup_factor: 8,
            merkle_tree_arity: 4,
            gpu_mode: ProgramImageGpuMode::Cuda,
        };

        let error = build_program_image_cache_proof_segment(Some(&cache))
            .expect_err("program image cache root should be canonical");

        assert_eq!(
            error,
            "program image cache tree root word 0 is non-canonical: non-canonical field element: 18446744069414584321"
        );
    }

    fn sample_block_rlp() -> Vec<u8> {
        let header_rlp = rlp_list(&legacy_header_items(
            hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
            None,
        ));
        let transactions = rlp_list(&[rlp_list(&[rlp_bytes(&[1])])]);
        let empty_list = rlp_list(&[]);
        rlp_list(&[header_rlp, transactions, empty_list])
    }

    fn legacy_header_items(
        transactions_root: [u8; 32],
        withdrawals_root: Option<[u8; 32]>,
    ) -> Vec<Vec<u8>> {
        let mut items = vec![
            rlp_bytes(&[0x11; 32]),
            rlp_bytes(&hex32(
                "1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
            )),
            rlp_bytes(&[0x33; 20]),
            rlp_bytes(&[0x44; 32]),
            rlp_bytes(&transactions_root),
            rlp_bytes(&[0x66; 32]),
            rlp_bytes(&[0x77; 256]),
            rlp_bytes(&[1]),
            rlp_bytes(&[2]),
            rlp_bytes(&[0x0f, 0x42, 0x40]),
            rlp_bytes(&[0x0d, 0xbb, 0xa0]),
            rlp_bytes(&[0x65]),
            rlp_bytes(b"lzvm"),
            rlp_bytes(&[0xaa; 32]),
            rlp_bytes(&[0xbb; 8]),
        ];
        if let Some(root) = withdrawals_root {
            items.push(rlp_bytes(&[1]));
            items.push(rlp_bytes(&root));
        }
        items
    }

    fn rlp_bytes(payload: &[u8]) -> Vec<u8> {
        if payload.len() == 1 && payload[0] <= 0x7f {
            return vec![payload[0]];
        }
        rlp_with_payload(0x80, 0xb7, payload)
    }

    fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
        let payload = items.iter().flatten().copied().collect::<Vec<_>>();
        rlp_with_payload(0xc0, 0xf7, &payload)
    }

    fn rlp_with_payload(short_base: u8, long_base: u8, payload: &[u8]) -> Vec<u8> {
        if payload.len() <= 55 {
            let mut output = vec![short_base + payload.len() as u8];
            output.extend_from_slice(payload);
            return output;
        }

        let length = length_bytes(payload.len());
        let mut output = vec![long_base + length.len() as u8];
        output.extend_from_slice(&length);
        output.extend_from_slice(payload);
        output
    }

    fn length_bytes(mut value: usize) -> Vec<u8> {
        let mut bytes = Vec::new();
        while value > 0 {
            bytes.push((value & 0xff) as u8);
            value >>= 8;
        }
        bytes.reverse();
        bytes
    }

    fn hex32(value: &str) -> [u8; 32] {
        hex_bytes(value)
            .try_into()
            .expect("hex string should be 32 bytes")
    }

    fn hex_bytes(value: &str) -> Vec<u8> {
        assert_eq!(value.len() % 2, 0);
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|chunk| {
                let text = std::str::from_utf8(chunk).expect("hex should be utf-8");
                u8::from_str_radix(text, 16).expect("hex byte should parse")
            })
            .collect()
    }
}

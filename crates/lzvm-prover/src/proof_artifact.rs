use lzvm_artifacts::eth_block_input::EthBlockInput;
use lzvm_artifacts::eth_block_input_segment::{
    encode_eth_block_input_segment, ETH_BLOCK_INPUT_SEGMENT_ID,
};
use lzvm_artifacts::key_directory::KeyDirectoryCatalog;
use lzvm_artifacts::pcs_nonce_segment::parse_pcs_query_nonce_segment;
use lzvm_artifacts::program_image::ProgramImageCommitmentCache;
use lzvm_artifacts::program_image_segment::{
    encode_program_image_cache_segment, PROGRAM_IMAGE_CACHE_SEGMENT_ID,
};
use lzvm_artifacts::proof::{ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{public_values_digest, PublicValues};
use lzvm_artifacts::witness_segment::WITNESS_COMMITMENT_SEGMENT_BASE_ID;
use lzvm_field::{Ext3, Felt};

use crate::contribution::{
    build_contribution_segment, build_witness_contribution_input, derive_worker_contribution_entry,
};
use crate::group_values::build_group_values_segment;
use crate::proof_values::build_pcs_proof_values_segment_from_packed_values;
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
    let binding_segments =
        build_proof_binding_segments(request.program_image_cache, request.eth_block_input)?;
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
            let final_query_challenge = values
                .first()
                .ok_or_else(|| "build FRI transcript values failed: no units".to_owned())?
                .commitments
                .final_query_challenge;
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
    }
    append_binding_segments(&mut segments, binding_segments);
    let proof = ProofArtifact {
        setup_hash: request.schedule.setup_hash,
        public_values_hash,
        segments,
    };
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
    let binding_segments =
        build_proof_binding_segments(request.program_image_cache, request.eth_block_input)?;
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
    if request.verify_outputs {
        validate_setup_preflight_hashes(request.catalog, &proof, public_values)
            .map_err(|error| format!("verify contribution proof output failed: {error}"))?;
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
    let binding_segments =
        build_proof_binding_segments(request.program_image_cache, request.eth_block_input)?;
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
    let proof_values_segment = build_pcs_proof_values_segment_from_packed_values(
        &request.catalog.layout.global_info,
        &request.auxiliary_inputs.proof_values,
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
    if request.verify_outputs {
        validate_setup_preflight_hashes(request.catalog, &proof, public_values)
            .map_err(|error| format!("verify contribution proof output failed: {error}"))?;
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
    let binding_segments =
        build_proof_binding_segments(request.program_image_cache, request.eth_block_input)?;
    let binding_segments_slice = binding_segments.as_slice();
    let proof_unit_values =
        collect_proof_unit_values(request.schedule, request.outputs, request.unit_values)?;
    let witness_outputs = request
        .outputs
        .iter()
        .map(|output| output.commitments())
        .collect::<Vec<_>>();
    let proof = if all_units_transcript_required(
        request.execution_units,
        request.outputs,
        request.auxiliary_inputs,
        request.evaluation_values_segment.is_some(),
    )? {
        build_witness_transcript_proof_artifact_for_all_units(
            request,
            public_values_hash,
            &witness_outputs,
            binding_segments_slice,
            &proof_unit_values,
        )?
    } else if binding_segments_slice.is_empty() {
        build_witness_proof_artifact(
            request.catalog,
            request.schedule,
            public_values_hash,
            &witness_outputs,
            &request.auxiliary_inputs.proof_values,
            &request.auxiliary_inputs.group_values,
            &proof_unit_values,
        )?
    } else {
        build_witness_proof_artifact_with_bindings(
            request.catalog,
            request.schedule,
            public_values_hash,
            &witness_outputs,
            ProofArtifactInputs {
                proof_values: &request.auxiliary_inputs.proof_values,
                group_values: &request.auxiliary_inputs.group_values,
                unit_values: &proof_unit_values,
                binding_segments: binding_segments_slice,
            },
        )?
    };
    let mut proof = proof;
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
    }
    append_binding_segments(&mut proof.segments, binding_segments);
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
    let data = encode_program_image_cache_segment(cache)
        .map_err(|error| format!("build program image cache segment failed: {error}"))?;
    Ok(Some(ProofSegment {
        id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
        data,
    }))
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

fn build_proof_binding_segments(
    cache: Option<&ProgramImageCommitmentCache>,
    eth_block_input: Option<&EthBlockInput>,
) -> Result<Vec<ProofSegment>, String> {
    let mut segments = Vec::new();
    if let Some(segment) = build_program_image_cache_proof_segment(cache)? {
        segments.push(segment);
    }
    if let Some(segment) = build_eth_block_input_proof_segment(eth_block_input)? {
        segments.push(segment);
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
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
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
            if auxiliary_inputs.evaluations.is_empty() && !has_evaluation_segment {
                return Err(format!(
                    "missing evaluation values for unit {unit_index}: expected {}",
                    execution_unit.expected_evaluation_value_count()
                ));
            }
        }
    }
    Ok(has_fri_unit || !auxiliary_inputs.evaluations.is_empty() || has_evaluation_segment)
}

fn build_witness_transcript_proof_artifact_for_all_units(
    request: &WitnessAllUnitsProofRequest<'_>,
    public_values_hash: [u8; 32],
    witness_outputs: &[&ProveWitnessCommitments],
    binding_segments: &[ProofSegment],
    unit_values: &[ProveUnitValues],
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

    let evaluation_segment = match request.evaluation_values_segment {
        Some(segment) => segment.clone(),
        None => {
            let evaluation_values = request
                .outputs
                .iter()
                .map(|output| ProvePcsEvaluationValues {
                    unit_index: output.commitments().unit_index(),
                    values: request.auxiliary_inputs.evaluations.clone(),
                })
                .collect::<Vec<_>>();
            build_pcs_evaluation_segment(request.schedule, &evaluation_values)
                .map_err(|error| format!("build evaluation segment failed: {error}"))?
        }
    };
    let transcript_auxiliary_inputs = request
        .outputs
        .iter()
        .map(|output| {
            let unit_index = output.commitments().unit_index();
            let mut inputs = output.auxiliary_inputs().clone();
            if let Some(values) = unit_values
                .iter()
                .find(|values| values.unit_index == unit_index)
            {
                inputs.unit_values = values.packed_values.clone();
            }
            inputs
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
                binding_segments,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let transcript_values =
        build_pcs_fri_transcript_values_from_trace_segments(request.schedule, &transcript_inputs)
            .map_err(|error| format!("build FRI transcript values failed: {error}"))?;
    let final_query_challenge = transcript_values
        .first()
        .ok_or_else(|| "build FRI transcript values failed: no units".to_owned())?
        .commitments
        .final_query_challenge;
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
        &request.auxiliary_inputs.proof_values,
    )
    .map_err(|error| format!("build proof values segment failed: {error}"))?;
    let group_values_segment = build_group_values_segment(
        &request.catalog.layout.global_info,
        &request.auxiliary_inputs.group_values,
    )
    .map_err(|error| format!("build group values segment failed: {error}"))?;
    let unit_values_segment = build_unit_values_segment_from_packed_values_batch(unit_values)
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

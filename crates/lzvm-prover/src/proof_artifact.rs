use lzvm_artifacts::key_directory::KeyDirectoryCatalog;
use lzvm_artifacts::proof::{ProofArtifact, ProofSegment};
use lzvm_field::{Ext3, Felt};

use crate::group_values::build_group_values_segment;
use crate::proof_values::build_pcs_proof_values_segment_from_packed_values;
use crate::unit_values::{build_unit_values_segment_from_packed_values_batch, ProveUnitValues};
use crate::{
    build_constant_opening_segment, build_pcs_material_manifest_segment,
    build_pcs_query_plan_segment_with_bindings, build_witness_commitment_segment,
    build_witness_opening_segment_batch, ProveSchedule, ProveWitnessCommitments,
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

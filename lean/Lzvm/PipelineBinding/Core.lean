/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AssumptionAudit
import Lzvm.EthBlockPublicInputBinding
import Lzvm.TraceConstraintArtifactBinding
import Lzvm.QueryPlanBinding

/-!
Runtime proof pipeline binding obligations.
-/

namespace Lzvm

structure RuntimePipelineBindingValidation (system : VerifierModel) where
  ethBindingValidation : RuntimeEthBlockPublicInputBindingValidation system
  traceBindingValidation : RuntimeTraceConstraintArtifactBindingValidation system
  queryPlanBindingValidation : RuntimeQueryPlanBindingValidation system
  pipelineBindingAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  artifactBindingValidationAgreement :
    RuntimeProofArtifactBindingValidationAgreement
      ethBindingValidation.proofArtifactBindingValidation
      queryPlanBindingValidation.challengeValidation.transcriptValidation.artifactBindingValidation
  pipelineBindingAcceptedImpliesEthBindingAccepted :
    forall artifact publicInput proof,
      pipelineBindingAccepted artifact publicInput proof ->
        ethBindingValidation.ethBlockBindingAccepted artifact publicInput proof
  pipelineBindingAcceptedImpliesTraceBindingAccepted :
    forall artifact publicInput proof,
      pipelineBindingAccepted artifact publicInput proof ->
        traceBindingValidation.traceArtifactBindingAccepted artifact publicInput proof
  pipelineBindingAcceptedImpliesQueryPlanBindingAccepted :
    forall artifact publicInput proof,
      pipelineBindingAccepted artifact publicInput proof ->
        queryPlanBindingValidation.queryPlanBindingAccepted artifact publicInput proof

def runtime_pipeline_trace_source_validation
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    ExternalSourceOpeningValidation system :=
  let openingValidation :=
    validation.traceBindingValidation.traceConstraintValidation.openingValidation
  openingValidation.runtimeSoundnessValidation.sourceValidation

def runtime_pipeline_trace_validation
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    RuntimeTraceConstraintValidation system :=
  validation.traceBindingValidation.traceConstraintValidation

def runtime_pipeline_opening_source_validation
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    ExternalSourceOpeningValidation system :=
  let openingValidation :=
    validation.queryPlanBindingValidation.openingValidation.openingValidation
  openingValidation.runtimeSoundnessValidation.sourceValidation

def runtime_pipeline_runtime_soundness_validation
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    RuntimeSoundnessValidation system :=
  let openingValidation :=
    validation.queryPlanBindingValidation.openingValidation.openingValidation
  openingValidation.runtimeSoundnessValidation

def runtime_pipeline_challenge_validation
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    RuntimeChallengeSegmentBindingValidation system :=
  validation.queryPlanBindingValidation.challengeValidation

def runtime_pipeline_transcript_validation
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    RuntimeTranscriptBindingValidation system :=
  (runtime_pipeline_challenge_validation validation).transcriptValidation

def runtime_pipeline_transcript_runtime_validation
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    RuntimeConformanceValidation system :=
  (runtime_pipeline_transcript_validation validation).artifactBindingValidation.runtimeValidation

def RuntimePipelineBindingEvidence
    (system : VerifierModel)
    (validation : RuntimePipelineBindingValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof)
    (requiresExternalSource : Prop) : Prop :=
  RuntimeEthBlockPublicInputBindingEvidence
      system
      validation.ethBindingValidation
      artifact
      publicInput
      proof
    /\ RuntimeProofArtifactBindingEvidence
      system
      validation.ethBindingValidation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
    /\ RuntimeArtifactEvidence
      system
      validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
      artifact
      publicInput
      proof
    /\ RuntimeTraceConstraintPreflightBindingEvidence
      system
      validation.traceBindingValidation
      artifact
      publicInput
      proof
    /\ RuntimeTraceConstraintEvidence
      system
      validation.traceBindingValidation.traceConstraintValidation
      artifact
      publicInput
      proof
      requiresExternalSource
    /\ RuntimeQueryPlanBindingEvidence
      system
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
    /\ RuntimeChallengeSegmentBindingEvidence
      system
      validation.queryPlanBindingValidation.challengeValidation
      artifact
      publicInput
      proof
    /\ RuntimeOpeningSegmentBindingEvidence
      system
      validation.queryPlanBindingValidation.openingValidation
      artifact
      publicInput
      proof
    /\ RuntimeOpeningEvidence
      system
      validation.queryPlanBindingValidation.openingValidation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
    /\ system.transcriptBound publicInput proof
    /\ system.publicInputBound publicInput proof
    /\ system.pcsOpeningsValid publicInput proof
    /\ system.friQueriesValid publicInput proof

theorem runtime_pipeline_binding_evidence_implies_transcript_bound
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      system.transcriptBound publicInput proof := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      _traceConstraintEvidence,
      _queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      _openingEvidence,
      transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact transcriptBound

theorem runtime_pipeline_binding_evidence_implies_public_input_bound
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      system.publicInputBound publicInput proof := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      _traceConstraintEvidence,
      _queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      _openingEvidence,
      _transcriptBound,
      publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact publicInputBound

theorem runtime_pipeline_binding_evidence_implies_pcs_and_fri
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      system.pcsOpeningsValid publicInput proof
        /\ system.friQueriesValid publicInput proof := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      _traceConstraintEvidence,
      _queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      _openingEvidence,
      _transcriptBound,
      _publicInputBound,
      pcsOpeningsValid,
      friQueriesValid⟩
  exact And.intro pcsOpeningsValid friQueriesValid

theorem runtime_pipeline_binding_evidence_implies_core_obligations
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RuntimeVerifierCoreContract system publicInput proof := by
  intro evidence
  exact
    And.intro
      (runtime_pipeline_binding_evidence_implies_transcript_bound evidence)
      (And.intro
        (runtime_pipeline_binding_evidence_implies_public_input_bound evidence)
        (runtime_pipeline_binding_evidence_implies_pcs_and_fri evidence))

theorem runtime_pipeline_binding_evidence_implies_runtime_artifact_core_contract
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RuntimeArtifactEvidence
          system
          validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
          artifact
          publicInput
          proof
        /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      runtimeArtifactEvidence,
      _tracePreflightEvidence,
      _traceConstraintEvidence,
      _queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      _openingEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid⟩
  have coreObligations :
      RuntimeVerifierCoreContract system publicInput proof :=
    And.intro transcriptBound
      (And.intro publicInputBound
        (And.intro pcsOpeningsValid friQueriesValid))
  exact
    And.intro
      runtimeArtifactEvidence
      coreObligations

theorem runtime_pipeline_binding_evidence_implies_external_source_requirements
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      ExternalSourceOpeningRequirement
          system
          (runtime_pipeline_trace_source_validation validation)
          publicInput
          proof
          requiresExternalSource
        /\ ExternalSourceOpeningRequirement
          system
          (runtime_pipeline_opening_source_validation validation)
          publicInput
          proof
          requiresExternalSource := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      traceConstraintEvidence,
      _queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact
    And.intro
      (runtime_opening_evidence_implies_external_source_requirement
        validation.traceBindingValidation.traceConstraintValidation.openingValidation
        artifact
        publicInput
        proof
        requiresExternalSource
        (runtime_trace_constraint_evidence_implies_opening_evidence
          validation.traceBindingValidation.traceConstraintValidation
          artifact
          publicInput
          proof
          requiresExternalSource
          traceConstraintEvidence))
      (runtime_opening_evidence_implies_external_source_requirement
        validation.queryPlanBindingValidation.openingValidation.openingValidation
        artifact
        publicInput
        proof
        requiresExternalSource
        openingEvidence)

theorem runtime_pipeline_binding_evidence_implies_seeded_query_plan_contract
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RuntimeQueryPlanBindingSeededContract
        system
        validation.queryPlanBindingValidation
        artifact
        publicInput
        proof := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      _traceConstraintEvidence,
      queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      _openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact
    runtime_query_plan_binding_evidence_implies_seeded_contract
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanEvidence

theorem runtime_pipeline_binding_evidence_implies_execution_obligations
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      exists witness trace constraints,
        system.traceConsistent publicInput proof trace
          /\ system.constraintsSatisfied constraints trace
          /\ system.witnessMatchesTrace witness trace := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      traceConstraintEvidence,
      _queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      _openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  have traceWitnessEvidence :=
    runtime_trace_constraint_evidence_implies_trace_witness_evidence
      validation.traceBindingValidation.traceConstraintValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      traceConstraintEvidence
  rcases traceWitnessEvidence with
    ⟨witness,
      trace,
      constraints,
      _traceExtracted,
      _constraintsEvaluated,
      _witnessExtracted,
      _backendConformant,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩
  exact
    Exists.intro witness
      (Exists.intro trace
        (Exists.intro constraints
          (And.intro traceConsistent
            (And.intro constraintsSatisfied witnessMatchesTrace))))

def RuntimePipelineBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimePipelineBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.pipelineBindingAccepted artifact publicInput proof

theorem runtime_pipeline_binding_checked_acceptance_eth
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation.ethBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.pipelineBindingAcceptedImpliesEthBindingAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_pipeline_binding_checked_acceptance_trace
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintArtifactBindingCheckedAcceptance
          system
          validation.traceBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.pipelineBindingAcceptedImpliesTraceBindingAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_pipeline_binding_checked_acceptance_query_plan
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation.queryPlanBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.pipelineBindingAcceptedImpliesQueryPlanBindingAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_pipeline_binding_checked_acceptance_segment_ids_unique
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let queryPlanValidation := validation.queryPlanBindingValidation
        let artifactValidation :=
          queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
        artifactValidation.proofSegmentIdsUnique artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_checked_acceptance_segment_ids_unique
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted

theorem runtime_pipeline_binding_checked_acceptance_container_canonical
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let queryPlanValidation := validation.queryPlanBindingValidation
        let artifactValidation :=
          queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
        artifactValidation.proofContainerCanonical artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_checked_acceptance_container_canonical
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted

theorem runtime_pipeline_binding_checked_acceptance_metadata_canonical
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let queryPlanValidation := validation.queryPlanBindingValidation
        let artifactValidation :=
          queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
        artifactValidation.proofMetadataCanonical artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_checked_acceptance_metadata_canonical
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted

theorem runtime_pipeline_binding_checked_acceptance_segment_payloads_nonempty
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let queryPlanValidation := validation.queryPlanBindingValidation
        let artifactValidation :=
          queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
        artifactValidation.proofSegmentPayloadsNonempty artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_checked_acceptance_segment_payloads_nonempty
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted

theorem runtime_pipeline_binding_checked_acceptance_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let queryPlanValidation := validation.queryPlanBindingValidation
        let artifactValidation :=
          queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
        artifactValidation.proofSegmentIdsAllowed artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_checked_acceptance_segment_ids_allowed
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted

theorem runtime_pipeline_binding_checked_acceptance_segments_present
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let queryPlanValidation := validation.queryPlanBindingValidation
        let artifactValidation :=
          queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
        artifactValidation.proofSegmentsPresent artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_checked_acceptance_segments_present
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted

theorem runtime_pipeline_binding_checked_acceptance_artifact_binding_validation_agreement
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let ethArtifactValidation :=
          validation.ethBindingValidation.proofArtifactBindingValidation
        let queryPlanValidation := validation.queryPlanBindingValidation
        let queryArtifactValidation :=
          queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
        RuntimeProofArtifactBindingValidationAgreement
          ethArtifactValidation
          queryArtifactValidation := by
  intro _artifact _publicInput _proof _accepted
  exact validation.artifactBindingValidationAgreement

theorem runtime_pipeline_binding_checked_acceptance_eth_artifact_wellformed_contract
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let artifactValidation :=
          validation.ethBindingValidation.proofArtifactBindingValidation
        artifactValidation.proofContainerCanonical artifact publicInput proof
          /\ artifactValidation.proofMetadataCanonical
            artifact
            publicInput
            proof
          /\ artifactValidation.proofSegmentsPresent
            artifact
            publicInput
            proof
          /\ artifactValidation.proofSegmentPayloadsNonempty
            artifact
            publicInput
            proof
          /\ artifactValidation.proofSegmentIdsAllowed
            artifact
            publicInput
            proof
          /\ artifactValidation.proofSegmentIdsUnique
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof accepted
  have ethAccepted :=
    runtime_pipeline_binding_checked_acceptance_eth
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_eth_block_public_input_binding_checked_acceptance_artifact_wellformed_contract
      validation.ethBindingValidation
      artifact
      publicInput
      proof
      ethAccepted

theorem runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation.queryPlanBindingValidation.openingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_checked_acceptance_opening
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted

theorem runtime_pipeline_binding_checked_acceptance_opening_segment_evidence
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingEvidence
          system
          validation.queryPlanBindingValidation.openingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have openingSegmentAccepted :=
    runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation.queryPlanBindingValidation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentAccepted

theorem runtime_pipeline_binding_checked_acceptance_verifier_accepts
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.accepts publicInput proof := by
  intro artifact publicInput proof accepted
  have ethAccepted :=
    runtime_pipeline_binding_checked_acceptance_eth
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAccepted :=
    runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding
      validation.ethBindingValidation
      artifact
      publicInput
      proof
      ethAccepted
  let proofArtifactValidation :=
    validation.ethBindingValidation.proofArtifactBindingValidation
  have runtimeAccepted :=
    proofArtifactValidation.bindingAcceptedImpliesRuntimeAccepted
      artifact
      publicInput
      proof
      artifactAccepted
  exact
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      proofArtifactValidation.runtimeValidation
      artifact
      publicInput
      proof
      runtimeAccepted

theorem runtime_pipeline_binding_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimePipelineBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have ethAccepted :=
    runtime_pipeline_binding_checked_acceptance_eth
      validation
      artifact
      publicInput
      proof
      accepted
  have traceAccepted :=
    runtime_pipeline_binding_checked_acceptance_trace
      validation
      artifact
      publicInput
      proof
      accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  have ethSound :=
    runtime_eth_block_public_input_binding_checked_acceptance_sound
      assumptions
      validation.ethBindingValidation
      artifact
      publicInput
      proof
      ethAccepted
  have traceSound :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_sound
      assumptions
      validation.traceBindingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      traceAccepted
  have queryPlanSound :=
    runtime_query_plan_binding_checked_acceptance_sound
      assumptions
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      queryPlanAccepted
  have ethEvidence := ethSound.left
  have artifactEvidence := ethSound.right.left
  have runtimeArtifactEvidence := ethSound.right.right.left
  have tracePreflightEvidence := traceSound.left
  have traceConstraintEvidence := traceSound.right.left
  have queryPlanEvidence := queryPlanSound.left
  have challengeEvidence := queryPlanSound.right.left
  have openingSegmentEvidence := queryPlanSound.right.right.left
  have openingEvidence := queryPlanSound.right.right.right.left
  have transcriptBound := queryPlanSound.right.right.right.right.left
  have pcsOpeningsValid := queryPlanSound.right.right.right.right.right.left
  have friQueriesValid := queryPlanSound.right.right.right.right.right.right.left
  have soundWitness := queryPlanSound.right.right.right.right.right.right.right
  have verifierAccepted :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have publicInputBound : system.publicInputBound publicInput proof :=
    assumptions.semantic.public_input_binding
      publicInput
      proof
      verifierAccepted
  exact
    And.intro
      (And.intro ethEvidence
        (And.intro artifactEvidence
          (And.intro runtimeArtifactEvidence
            (And.intro tracePreflightEvidence
              (And.intro traceConstraintEvidence
                (And.intro queryPlanEvidence
                  (And.intro challengeEvidence
                    (And.intro openingSegmentEvidence
                      (And.intro openingEvidence
                        (And.intro transcriptBound
                          (And.intro publicInputBound
                            (And.intro pcsOpeningsValid friQueriesValid))))))))))))
      soundWitness

theorem runtime_pipeline_binding_checked_acceptance_pipeline_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimePipelineBindingEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have sound :=
    runtime_pipeline_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact sound.left

theorem runtime_pipeline_binding_checked_acceptance_audited_assumptions
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RuntimePipelineBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have audited :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have sound :=
    runtime_pipeline_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact And.intro audited sound

theorem runtime_pipeline_binding_checked_acceptance_transcript_bound_without_assumptions
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.transcriptBound publicInput proof := by
  intro artifact publicInput proof accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  have challengeAccepted :=
    runtime_query_plan_binding_checked_acceptance_challenge
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted
  have transcriptAccepted :=
    runtime_challenge_segment_binding_checked_acceptance_transcript
      validation.queryPlanBindingValidation.challengeValidation
      artifact
      publicInput
      proof
      challengeAccepted
  exact
    runtime_transcript_binding_checked_acceptance_transcript_bound
      validation.queryPlanBindingValidation.challengeValidation.transcriptValidation
      artifact
      publicInput
      proof
      transcriptAccepted

theorem runtime_pipeline_binding_checked_acceptance_transcript_bound
    {system : VerifierModel}
    (_assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.transcriptBound publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_pipeline_binding_checked_acceptance_transcript_bound_without_assumptions
      validation
      artifact
      publicInput
      proof
      accepted

theorem runtime_pipeline_binding_checked_acceptance_public_input_bound_from_semantic_assumptions
    {system : VerifierModel}
    (semanticAssumptions : SemanticAssumptions system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.publicInputBound publicInput proof := by
  intro artifact publicInput proof accepted
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  exact semanticAssumptions.public_input_binding publicInput proof verifierAccepts

theorem runtime_pipeline_binding_checked_acceptance_public_input_bound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.publicInputBound publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_pipeline_binding_checked_acceptance_public_input_bound_from_semantic_assumptions
      assumptions.semantic
      validation
      artifact
      publicInput
      proof
      accepted

theorem runtime_pipeline_binding_checked_acceptance_query_plan_pcs_and_fri
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_checked_acceptance_pcs_and_fri
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted

theorem runtime_pipeline_binding_checked_acceptance_pcs_and_fri_without_assumptions
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_pipeline_binding_checked_acceptance_query_plan_pcs_and_fri
      validation
      artifact
      publicInput
      proof
      accepted

theorem runtime_pipeline_binding_checked_acceptance_pcs_and_fri
    {system : VerifierModel}
    (_assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_pipeline_binding_checked_acceptance_pcs_and_fri_without_assumptions
      validation
      artifact
      publicInput
      proof
      accepted


end Lzvm

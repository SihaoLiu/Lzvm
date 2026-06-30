/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AssumptionAudit
import Lzvm.EthBlockPublicInputBinding
import Lzvm.FramedGuestInputBinding
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

theorem runtime_pipeline_binding_evidence_implies_trace_preflight_evidence
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
      RuntimeTraceConstraintPreflightBindingEvidence
        system
        validation.traceBindingValidation
        artifact
        publicInput
        proof := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      tracePreflightEvidence,
      _traceConstraintEvidence,
      _queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      _openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact tracePreflightEvidence

theorem runtime_pipeline_binding_evidence_implies_trace_constraint_evidence
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
      RuntimeTraceConstraintEvidence
        system
        validation.traceBindingValidation.traceConstraintValidation
        artifact
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
      _openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact traceConstraintEvidence

theorem runtime_pipeline_binding_evidence_implies_query_plan_evidence
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
      RuntimeQueryPlanBindingEvidence
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
  exact queryPlanEvidence

theorem runtime_pipeline_binding_evidence_implies_challenge_evidence
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
      RuntimeChallengeSegmentBindingEvidence
        system
        validation.queryPlanBindingValidation.challengeValidation
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
      _queryPlanEvidence,
      challengeEvidence,
      _openingSegmentEvidence,
      _openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact challengeEvidence

theorem runtime_pipeline_binding_evidence_implies_opening_segment_evidence
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
      RuntimeOpeningSegmentBindingEvidence
        system
        validation.queryPlanBindingValidation.openingValidation
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
      _queryPlanEvidence,
      _challengeEvidence,
      openingSegmentEvidence,
      _openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact openingSegmentEvidence

theorem runtime_pipeline_binding_evidence_implies_opening_evidence
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
      RuntimeOpeningEvidence
        system
        validation.queryPlanBindingValidation.openingValidation.openingValidation
        artifact
        publicInput
        proof
        requiresExternalSource := by
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
      openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact openingEvidence

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

theorem runtime_pipeline_binding_evidence_implies_trace_semantic_evidence_complete
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
      RuntimeTraceConstraintSemanticEvidenceComplete
        system
        validation.traceBindingValidation.traceConstraintValidation
        artifact
        publicInput
        proof := by
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
  exact
    runtime_trace_constraint_evidence_implies_semantic_evidence_complete
      validation.traceBindingValidation.traceConstraintValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      traceConstraintEvidence

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

structure RuntimePipelineFramedGuestInputBindingBridge
    (system : VerifierModel)
    (validation : RuntimePipelineBindingValidation system) where
  framedGuestInputBindingValidation : RuntimeFramedGuestInputBindingValidation system
  pipelineBindingAcceptedImpliesFramedGuestInputAccepted :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          framedGuestInputBindingValidation
          artifact
          publicInput
          proof

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

theorem runtime_pipeline_binding_checked_acceptance_framed_guest_input
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system)
    (bridge : RuntimePipelineFramedGuestInputBindingBridge system validation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          bridge.framedGuestInputBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    bridge.pipelineBindingAcceptedImpliesFramedGuestInputAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_pipeline_binding_checked_acceptance_framed_guest_input_segment_payload_nonempty
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system)
    (bridge : RuntimePipelineFramedGuestInputBindingBridge system validation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        bridge.framedGuestInputBindingValidation.framedGuestInputProofSegmentPayloadNonempty
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have framedAccepted :=
    runtime_pipeline_binding_checked_acceptance_framed_guest_input
      validation
      bridge
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_framed_guest_input_binding_checked_acceptance_segment_payload_nonempty
      bridge.framedGuestInputBindingValidation
      artifact
      publicInput
      proof
      framedAccepted

theorem runtime_pipeline_binding_checked_acceptance_framed_guest_input_co_bindings
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system)
    (bridge : RuntimePipelineFramedGuestInputBindingBridge system validation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        bridge.framedGuestInputBindingValidation.framedGuestInputCoBoundWithEthBlock
            artifact
            publicInput
            proof
          /\ bridge.framedGuestInputBindingValidation.framedGuestInputCoBoundWithProgramImage
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof accepted
  have framedAccepted :=
    runtime_pipeline_binding_checked_acceptance_framed_guest_input
      validation
      bridge
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro
      (runtime_framed_guest_input_binding_checked_acceptance_eth_block_co_binding
        bridge.framedGuestInputBindingValidation
        artifact
        publicInput
        proof
        framedAccepted)
      (runtime_framed_guest_input_binding_checked_acceptance_program_image_cache_co_binding
        bridge.framedGuestInputBindingValidation
        artifact
        publicInput
        proof
        framedAccepted)

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

theorem runtime_pipeline_binding_checked_acceptance_trace_preflight_evidence
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintPreflightBindingEvidence
          system
          validation.traceBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have traceAccepted :=
    runtime_pipeline_binding_checked_acceptance_trace
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_trace_constraint_artifact_binding_checked_acceptance_evidence
      validation.traceBindingValidation
      artifact
      publicInput
      proof
      traceAccepted

theorem runtime_pipeline_binding_checked_acceptance_trace_payload_valid
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.traceBindingValidation.traceConstraintSegmentPayloadValid
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have preflightEvidence :=
    runtime_pipeline_binding_checked_acceptance_trace_preflight_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_trace_constraint_preflight_binding_evidence_implies_payload_valid
      validation.traceBindingValidation
      artifact
      publicInput
      proof
      preflightEvidence

theorem runtime_pipeline_binding_checked_acceptance_trace_witness_segments_match
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.traceBindingValidation.witnessCommitmentSegmentsMatchTraceEvidence
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have preflightEvidence :=
    runtime_pipeline_binding_checked_acceptance_trace_preflight_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_trace_constraint_preflight_binding_evidence_implies_witness_segments_match
      validation.traceBindingValidation
      artifact
      publicInput
      proof
      preflightEvidence

theorem runtime_pipeline_binding_checked_acceptance_trace_constraint_catalog_matches
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.traceBindingValidation.constraintCatalogMatchesTraceEvidence
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have preflightEvidence :=
    runtime_pipeline_binding_checked_acceptance_trace_preflight_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_trace_constraint_preflight_binding_evidence_implies_constraint_catalog_matches
      validation.traceBindingValidation
      artifact
      publicInput
      proof
      preflightEvidence

theorem runtime_pipeline_binding_checked_acceptance_trace_semantic_evidence_complete
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintSemanticEvidenceComplete
          system
          validation.traceBindingValidation.traceConstraintValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have traceAccepted :=
    runtime_pipeline_binding_checked_acceptance_trace
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_trace_constraint_artifact_binding_checked_acceptance_semantic_evidence_complete
      validation.traceBindingValidation
      artifact
      publicInput
      proof
      traceAccepted

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

theorem runtime_pipeline_binding_checked_acceptance_query_plan_material_manifest_contract
    {system : VerifierModel} (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance system validation artifact publicInput proof ->
        RuntimeQueryPlanMaterialManifestContract
          system
          validation.queryPlanBindingValidation
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
    runtime_query_plan_binding_checked_acceptance_material_manifest_contract
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted

theorem runtime_pipeline_binding_checked_acceptance_artifact_finalized
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
        RuntimeProofArtifactFinalized
          system
          artifactValidation
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
    runtime_query_plan_binding_checked_acceptance_artifact_finalized
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted

theorem runtime_pipeline_binding_checked_acceptance_artifact_structural_obligations
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
        RuntimeProofArtifactBindingStructuralObligations
          system
          artifactValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have artifactFinalized :=
    runtime_pipeline_binding_checked_acceptance_artifact_finalized
      validation
      artifact
      publicInput
      proof
      accepted
  let queryPlanValidation := validation.queryPlanBindingValidation
  let artifactValidation :=
    queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
  exact
    runtime_proof_artifact_finalized_structural_obligations
      artifactValidation
      artifact
      publicInput
      proof
      artifactFinalized

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
  have artifactStructural :=
    runtime_pipeline_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.right.right.right.right.left

theorem runtime_pipeline_binding_checked_acceptance_unit_values_trace_identity_coverage
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
        artifactValidation.proofUnitValuesTraceIdentityCoverage artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have artifactStructural :=
    runtime_pipeline_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.right.right.right.right.right

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
  have artifactStructural :=
    runtime_pipeline_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.left

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
  have artifactStructural :=
    runtime_pipeline_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.left

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
  have artifactStructural :=
    runtime_pipeline_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.right.right.left

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
  have artifactStructural :=
    runtime_pipeline_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.right.right.right.left

theorem runtime_pipeline_binding_checked_acceptance_concrete_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system)
    (binding :
      let queryPlanValidation := validation.queryPlanBindingValidation
      let challengeValidation := queryPlanValidation.challengeValidation
      RuntimeProofArtifactConcreteSegmentIdBinding
        challengeValidation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  have queryPlanAccepted :=
    validation.pipelineBindingAcceptedImpliesQueryPlanBindingAccepted
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_checked_acceptance_concrete_segment_ids_allowed
      validation.queryPlanBindingValidation
      binding
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
  have artifactStructural :=
    runtime_pipeline_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.right.left

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
            proof
          /\ artifactValidation.proofUnitValuesTraceIdentityCoverage
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

theorem runtime_pipeline_binding_checked_acceptance_eth_concrete_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.ethBindingValidation.proofArtifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_eth_block_public_input_binding_checked_acceptance_concrete_segment_ids_allowed
      validation.ethBindingValidation
      binding
      artifact
      publicInput
      proof
      (runtime_pipeline_binding_checked_acceptance_eth
        validation
        artifact
        publicInput
        proof
        accepted)

theorem runtime_pipeline_binding_eth_concrete_segment_id_binding_of_query_binding
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system)
    (binding :
      let queryPlanValidation := validation.queryPlanBindingValidation
      let challengeValidation := queryPlanValidation.challengeValidation
      RuntimeProofArtifactConcreteSegmentIdBinding
        challengeValidation.transcriptValidation.artifactBindingValidation) :
    RuntimeProofArtifactConcreteSegmentIdBinding
      validation.ethBindingValidation.proofArtifactBindingValidation := by
  exact
    runtime_proof_artifact_concrete_segment_id_binding_of_agreement_left
      validation.artifactBindingValidationAgreement
      binding

theorem
    runtime_pipeline_binding_checked_acceptance_eth_concrete_segment_ids_allowed_of_query_binding
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system)
    (binding :
      let queryPlanValidation := validation.queryPlanBindingValidation
      let challengeValidation := queryPlanValidation.challengeValidation
      RuntimeProofArtifactConcreteSegmentIdBinding
        challengeValidation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_pipeline_binding_checked_acceptance_eth_concrete_segment_ids_allowed
      validation
      (runtime_pipeline_binding_eth_concrete_segment_id_binding_of_query_binding
        validation
        binding)
      artifact
      publicInput
      proof
      accepted

theorem runtime_pipeline_binding_checked_acceptance_eth_full_contract
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
          /\ RuntimeEthBlockPublicInputBindingStructuralObligations
            system
            validation.ethBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have ethAccepted :=
    runtime_pipeline_binding_checked_acceptance_eth
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_eth_block_public_input_binding_checked_acceptance_full_contract
      assumptions
      validation.ethBindingValidation
      artifact
      publicInput
      proof
      ethAccepted

theorem runtime_pipeline_binding_checked_acceptance_framed_guest_input_soundness_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    (bridge : RuntimePipelineFramedGuestInputBindingBridge system validation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFramedGuestInputBindingSoundnessContract
          system
          bridge.framedGuestInputBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have framedAccepted :=
    runtime_pipeline_binding_checked_acceptance_framed_guest_input
      validation
      bridge
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_framed_guest_input_binding_checked_acceptance_soundness_contract
      assumptions
      bridge.framedGuestInputBindingValidation
      artifact
      publicInput
      proof
      framedAccepted

theorem runtime_pipeline_binding_checked_acceptance_framed_guest_input_concrete_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system)
    (bridge : RuntimePipelineFramedGuestInputBindingBridge system validation)
    (binding :
      let framedValidation := bridge.framedGuestInputBindingValidation
      let inputValidation := framedValidation.ethBlockValidation
      RuntimeProofArtifactConcreteSegmentIdBinding
        inputValidation.proofArtifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_framed_guest_input_binding_checked_acceptance_concrete_segment_ids_allowed
      bridge.framedGuestInputBindingValidation
      binding
      artifact
      publicInput
      proof
      (runtime_pipeline_binding_checked_acceptance_framed_guest_input
        validation
        bridge
        artifact
        publicInput
        proof
        accepted)

theorem runtime_pipeline_binding_checked_acceptance_framed_guest_input_full_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    (bridge : RuntimePipelineFramedGuestInputBindingBridge system validation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFramedGuestInputBindingEvidence
            system
            bridge.framedGuestInputBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeFramedGuestInputBindingStructuralObligations
            system
            bridge.framedGuestInputBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have framedAccepted :=
    runtime_pipeline_binding_checked_acceptance_framed_guest_input
      validation
      bridge
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_framed_guest_input_binding_checked_acceptance_full_contract
      assumptions
      bridge.framedGuestInputBindingValidation
      artifact
      publicInput
      proof
      framedAccepted

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

theorem runtime_pipeline_binding_checked_acceptance_fri_parser_contract
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system)
    (boundary :
      RuntimeFriOpeningSegmentParserBoundary
        system
        validation.queryPlanBindingValidation.openingValidation) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFriOpeningSegmentParserContract
          boundary
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
    runtime_opening_segment_binding_checked_acceptance_fri_parser_contract
      validation.queryPlanBindingValidation.openingValidation
      boundary
      artifact
      publicInput
      proof
      openingSegmentAccepted

theorem runtime_pipeline_binding_checked_acceptance_fri_fold_trace_identity_contract
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFriFoldTraceIdentityContract
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
    runtime_opening_segment_binding_checked_acceptance_fri_fold_trace_identity_contract
      validation.queryPlanBindingValidation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentAccepted

theorem runtime_pipeline_binding_checked_acceptance_fri_fold_query_plan_order_contract
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFriFoldQueryPlanOrderContract
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
    runtime_opening_segment_binding_checked_acceptance_fri_fold_query_plan_order_contract
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
  have ethFull :=
    runtime_pipeline_binding_checked_acceptance_eth_full_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have traceSound :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_sound
      assumptions
      validation.traceBindingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      traceAccepted
  have queryPlanFull :=
    runtime_query_plan_binding_checked_acceptance_full_soundness_contract
      assumptions
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      queryPlanAccepted
  rcases queryPlanFull with
    ⟨queryPlanEvidence,
      _queryPlanBound,
      challengeEvidence,
      openingSegmentEvidence,
      _openingSegmentBound,
      openingEvidence,
      _openingBound,
      transcriptBound,
      pcsOpeningsValid,
      friQueriesValid,
      _queryPlanCoreContract,
      soundWitness⟩
  have ethEvidence := ethFull.left
  have artifactEvidence := ethFull.right.left
  have runtimeArtifactEvidence := ethFull.right.right.right.left
  have tracePreflightEvidence := traceSound.left
  have traceConstraintEvidence := traceSound.right.left
  have verifierAccepted :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have publicInputBound : system.publicInputBound publicInput proof :=
    assumption_bundle_public_input_binding
      assumptions
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


end Lzvm

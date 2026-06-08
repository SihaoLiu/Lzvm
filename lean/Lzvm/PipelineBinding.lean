/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AssumptionAudit
import Lzvm.EthBlockPublicInputBinding
import Lzvm.TraceConstraintArtifactBinding
import Lzvm.QueryPlanBinding
import Lzvm.DigestPrefix

/-!
Runtime proof pipeline binding obligations.
-/

namespace Lzvm

structure RuntimePipelineBindingValidation (system : VerifierModel) where
  ethBindingValidation : RuntimeEthBlockPublicInputBindingValidation system
  traceBindingValidation : RuntimeTraceConstraintArtifactBindingValidation system
  queryPlanBindingValidation : RuntimeQueryPlanBindingValidation system
  pipelineBindingAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
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
  exact evidence.right.right.right.right.right.right.right.right.right.left

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
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid⟩
  exact ⟨transcriptBound, publicInputBound, pcsOpeningsValid, friQueriesValid⟩

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
    ⟨witness,
      trace,
      constraints,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩

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
  have publicInputBound : system.publicInputBound publicInput proof := by
    rcases soundWitness with
      ⟨_witness,
        _trace,
        _constraints,
        _transcriptBound,
        publicInputBound,
        _pcsOpeningsValid,
        _friQueriesValid,
        _traceConsistent,
        _constraintsSatisfied,
        _witnessMatchesTrace⟩
    exact publicInputBound
  exact
    ⟨⟨ethEvidence,
        artifactEvidence,
        runtimeArtifactEvidence,
        tracePreflightEvidence,
        traceConstraintEvidence,
        queryPlanEvidence,
        challengeEvidence,
        openingSegmentEvidence,
        openingEvidence,
        transcriptBound,
        publicInputBound,
        pcsOpeningsValid,
        friQueriesValid⟩,
      soundWitness⟩

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

theorem runtime_pipeline_binding_evidence_implies_runtime_soundness_evidence
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
      RuntimeSoundnessEvidence
        system
        (runtime_pipeline_runtime_soundness_validation validation)
        artifact
        publicInput
        proof
        requiresExternalSource := by
  intro evidence
  exact evidence.right.right.right.right.right.right.right.right.left.left

theorem runtime_pipeline_binding_required_external_source_sound
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
        requiresExternalSource ->
          RuntimePipelineBindingEvidence
              system
              validation
              artifact
              publicInput
              proof
              requiresExternalSource
            /\ ExternalSourceOpeningEvidence
              system
              (runtime_pipeline_trace_source_validation validation)
              publicInput
              proof
            /\ ExternalSourceOpeningEvidence
              system
              (runtime_pipeline_opening_source_validation validation)
              publicInput
              proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have pipelineSound :=
    runtime_pipeline_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have traceAccepted :=
    runtime_pipeline_binding_checked_acceptance_trace
      validation
      artifact
      publicInput
      proof
      accepted
  have traceConstraintAccepted :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_trace_constraint
      validation.traceBindingValidation
      artifact
      publicInput
      proof
      traceAccepted
  have traceRequired :=
    runtime_trace_constraint_required_external_source_pcs_sound
      assumptions
      validation.traceBindingValidation.traceConstraintValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      traceConstraintAccepted
      required
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  have openingSegmentAccepted :=
    runtime_query_plan_binding_checked_acceptance_opening
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted
  have openingAccepted :=
    runtime_opening_segment_binding_checked_acceptance_opening
      validation.queryPlanBindingValidation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentAccepted
  have openingRequired :=
    runtime_opening_required_external_source_sound
      assumptions
      validation.queryPlanBindingValidation.openingValidation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
      required
  exact
    ⟨pipelineSound.left,
      traceRequired.left,
      openingRequired.right.left,
      traceRequired.right.left,
      pipelineSound.right⟩

theorem runtime_pipeline_binding_checked_acceptance_core_obligations
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
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_pipeline_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  exact runtime_pipeline_binding_evidence_implies_core_obligations sound.left

theorem runtime_pipeline_binding_checked_acceptance_query_opening_evidence
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
        RuntimeQueryPlanBindingEvidence
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
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
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
  rcases sound.left with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      _traceConstraintEvidence,
      queryPlanEvidence,
      challengeEvidence,
      openingSegmentEvidence,
      openingEvidence,
      transcriptBound,
      _publicInputBound,
      pcsOpeningsValid,
      friQueriesValid⟩
  exact
    ⟨queryPlanEvidence,
      challengeEvidence,
      openingSegmentEvidence,
      openingEvidence,
      transcriptBound,
      pcsOpeningsValid,
      friQueriesValid⟩

theorem runtime_pipeline_binding_checked_acceptance_query_opening_contract
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
        RuntimeQueryPlanBindingEvidence
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
          /\ (system.transcriptBound publicInput proof
            /\ system.publicInputBound publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof)
          /\ SoundWitness system publicInput proof := by
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
  cases sound.left with
  | intro _ethEvidence tail =>
    cases tail with
    | intro _artifactEvidence tail =>
      cases tail with
      | intro _runtimeArtifactEvidence tail =>
        cases tail with
        | intro _tracePreflightEvidence tail =>
          cases tail with
          | intro _traceConstraintEvidence tail =>
            cases tail with
            | intro queryPlanEvidence tail =>
              cases tail with
              | intro challengeEvidence tail =>
                cases tail with
                | intro openingSegmentEvidence tail =>
                  cases tail with
                  | intro openingEvidence tail =>
                    cases tail with
                    | intro transcriptBound tail =>
                      cases tail with
                      | intro publicInputBound tail =>
                        cases tail with
                        | intro pcsOpeningsValid friQueriesValid =>
                          exact
                            And.intro queryPlanEvidence
                              (And.intro challengeEvidence
                                (And.intro openingSegmentEvidence
                                  (And.intro openingEvidence
                                      (And.intro
                                        (And.intro transcriptBound
                                          (And.intro publicInputBound
                                            (And.intro pcsOpeningsValid friQueriesValid)))
                                      sound.right))))

theorem runtime_pipeline_binding_checked_acceptance_opening_segment_bound_contract
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
        RuntimeOpeningSegmentBindingBoundContract
          system
          validation.queryPlanBindingValidation.openingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have contract :=
    runtime_pipeline_binding_checked_acceptance_query_opening_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  cases contract with
  | intro _queryPlanEvidence tail =>
    cases tail with
    | intro _challengeEvidence tail =>
      cases tail with
      | intro openingSegmentEvidence _tail =>
        exact
          runtime_opening_segment_binding_evidence_implies_bound_contract
            validation.queryPlanBindingValidation.openingValidation
            artifact
            publicInput
            proof
            openingSegmentEvidence

theorem runtime_pipeline_binding_checked_acceptance_challenge_query_opening_contract
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
        (runtime_pipeline_challenge_validation validation).challengeSegmentPayloadValid
            artifact
            publicInput
            proof
          /\ (runtime_pipeline_challenge_validation validation).challengeSegmentMatchesTranscript
            artifact
            publicInput
            proof
          /\ (runtime_pipeline_transcript_validation validation).challengeSegmentBound
            artifact
            publicInput
            proof
          /\ (runtime_pipeline_transcript_validation validation).queryPlanBound
            artifact
            publicInput
            proof
          /\ validation.queryPlanBindingValidation.openingValidation.queryPlanBound
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
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have contract :=
    runtime_pipeline_binding_checked_acceptance_query_opening_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  cases contract with
  | intro queryPlanEvidence tail =>
    cases tail with
    | intro challengeEvidence tail =>
      cases tail with
      | intro _openingSegmentEvidence tail =>
        cases tail with
        | intro openingEvidence tail =>
          cases tail with
          | intro obligations soundWitness =>
            cases queryPlanEvidence with
            | intro _segmentCanonical tail =>
              cases tail with
              | intro _derivedFromTranscript tail =>
                cases tail with
                | intro _matchesOpenedArtifacts tail =>
                  cases tail with
                  | intro transcriptQueryPlanBound openingQueryPlanBound =>
                    cases challengeEvidence with
                    | intro challengePayloadValid tail =>
                      cases tail with
                      | intro challengeMatchesTranscript challengeSegmentBound =>
                        cases obligations with
                        | intro transcriptBound tail =>
                          cases tail with
                          | intro _publicInputBound tail =>
                            cases tail with
                            | intro pcsOpeningsValid friQueriesValid =>
                              exact
                                And.intro challengePayloadValid
                                  (And.intro challengeMatchesTranscript
                                    (And.intro challengeSegmentBound
                                      (And.intro transcriptQueryPlanBound
                                        (And.intro openingQueryPlanBound
                                          (And.intro openingEvidence
                                            (And.intro transcriptBound
                                              (And.intro pcsOpeningsValid
                                                (And.intro friQueriesValid soundWitness))))))))

theorem runtime_pipeline_compact_digest_merkle_observation_eq_full_state
    {alpha : Type u}
    (evidence : DigestPrefixRoundEvidence alpha) :
    DigestPrefixMerkleObservation (DigestPrefixRoundVisibleWords evidence) =
      FullStateMerkleObservation evidence.fullStateWords := by
  exact digest_prefix_round_merkle_observation_eq_full_state evidence

theorem runtime_pipeline_binding_checked_acceptance_compact_digest_merkle_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    (digestValidation : RowMajorDigestPrefixValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RowMajorDigestPrefixEvidence
            system
            digestValidation
            publicInput
            proof ->
          digestValidation.leafValidation.wideLinearDigestsBindRows publicInput proof
          /\ RuntimeOpeningEvidence
            system
            validation.queryPlanBindingValidation.openingValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted digestEvidence
  have wideLinearDigests :=
    row_major_digest_prefix_evidence_implies_wide_linear_digests
      digestValidation
      publicInput
      proof
      digestEvidence
  have contract :=
    runtime_pipeline_binding_checked_acceptance_challenge_query_opening_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  cases contract with
  | intro _challengePayloadValid tail =>
    cases tail with
    | intro _challengeMatchesTranscript tail =>
      cases tail with
      | intro _challengeSegmentBound tail =>
        cases tail with
        | intro _transcriptQueryPlanBound tail =>
          cases tail with
          | intro _openingQueryPlanBound tail =>
            cases tail with
            | intro openingEvidence tail =>
              cases tail with
              | intro _transcriptBound tail =>
                cases tail with
                | intro pcsOpeningsValid tail =>
                  cases tail with
                  | intro friQueriesValid soundWitness =>
                    exact
                      And.intro wideLinearDigests
                        (And.intro openingEvidence
                          (And.intro pcsOpeningsValid
                            (And.intro friQueriesValid soundWitness)))

theorem runtime_pipeline_binding_checked_acceptance_soundness_obligations
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
        RuntimeArtifactSoundnessObligations
          system
          validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_pipeline_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  have runtimeArtifactEvidence := sound.left.right.right.left
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have coreObligations :=
    runtime_pipeline_binding_evidence_implies_core_obligations sound.left
  exact
    ⟨runtimeArtifactEvidence, verifierAccepts, coreObligations⟩

theorem runtime_pipeline_binding_checked_acceptance_execution_obligations
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
        exists witness trace constraints,
          system.traceConsistent publicInput proof trace
            /\ system.constraintsSatisfied constraints trace
            /\ system.witnessMatchesTrace witness trace := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_pipeline_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  exact runtime_pipeline_binding_evidence_implies_execution_obligations sound.left

theorem runtime_pipeline_binding_checked_acceptance_trace_conformance_contract
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
        RuntimeTraceConstraintEvidence
            system
            (runtime_pipeline_trace_validation validation)
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ (exists witness trace constraints,
            (runtime_pipeline_trace_validation validation).traceExtracted
              artifact
              publicInput
              proof
              trace
              /\ (runtime_pipeline_trace_validation validation).constraintsEvaluated
                artifact
                publicInput
                proof
                constraints
                trace
              /\ (runtime_pipeline_trace_validation validation).witnessExtractedFromTrace
                artifact
                publicInput
                proof
                witness
                trace
              /\ (runtime_pipeline_trace_validation validation).constraintBackendConformant
                artifact
                publicInput
                proof
                constraints
                trace
              /\ system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ system.pcsOpeningsValid publicInput proof
          /\ SoundWitness system publicInput proof := by
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
  cases sound.left with
  | intro _ethEvidence tail =>
    cases tail with
    | intro _artifactEvidence tail =>
      cases tail with
      | intro _runtimeArtifactEvidence tail =>
        cases tail with
        | intro _tracePreflightEvidence tail =>
          cases tail with
          | intro traceConstraintEvidence tail =>
            cases tail with
            | intro _queryPlanEvidence tail =>
              cases tail with
              | intro _challengeEvidence tail =>
                cases tail with
                | intro _openingSegmentEvidence tail =>
                  cases tail with
                  | intro _openingEvidence tail =>
                    cases tail with
                    | intro _transcriptBound tail =>
                      cases tail with
                      | intro _publicInputBound tail =>
                        cases tail with
                        | intro pcsOpeningsValid _friQueriesValid =>
                          have traceWitnessEvidence :=
                            runtime_trace_constraint_evidence_implies_trace_witness_evidence
                              (runtime_pipeline_trace_validation validation)
                              artifact
                              publicInput
                              proof
                              requiresExternalSource
                              traceConstraintEvidence
                          exact
                            And.intro traceConstraintEvidence
                              (And.intro traceWitnessEvidence
                                (And.intro pcsOpeningsValid sound.right))

theorem runtime_pipeline_binding_checked_acceptance_verifier_sound_witness
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
        system.accepts publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have sound :=
    runtime_pipeline_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  exact ⟨verifierAccepts, sound.right⟩

theorem runtime_pipeline_binding_checked_acceptance_verifier_core_contract
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
        system.accepts publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have coreObligations :=
    runtime_pipeline_binding_checked_acceptance_core_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact And.intro verifierAccepts coreObligations

theorem runtime_pipeline_binding_checked_acceptance_runtime_soundness_contract
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
        RuntimeSoundnessEvidence
            system
            (runtime_pipeline_runtime_soundness_validation validation)
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
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
  have runtimeEvidence :=
    runtime_pipeline_binding_evidence_implies_runtime_soundness_evidence
      sound.left
  have coreObligations :=
    runtime_pipeline_binding_checked_acceptance_core_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact And.intro runtimeEvidence (And.intro coreObligations sound.right)

theorem runtime_pipeline_binding_checked_acceptance_full_soundness_contract
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
          /\ RuntimeArtifactSoundnessObligations
            system
            validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
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
  have artifactObligations :=
    runtime_pipeline_binding_checked_acceptance_soundness_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have coreObligations :=
    runtime_pipeline_binding_checked_acceptance_core_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have executionObligations :=
    runtime_pipeline_binding_checked_acceptance_execution_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    ⟨sound.left,
      artifactObligations,
      coreObligations,
      executionObligations,
      sound.right⟩

end Lzvm

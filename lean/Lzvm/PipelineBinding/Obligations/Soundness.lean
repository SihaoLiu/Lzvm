/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.PipelineBinding.Obligations.Core

/-!
Runtime proof pipeline binding soundness obligations.
-/

namespace Lzvm

universe u

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

theorem runtime_pipeline_binding_checked_acceptance_runtime_artifact_soundness_obligations
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
  have runtimeArtifactEvidence :=
    runtime_artifact_checked_acceptance_evidence
      proofArtifactValidation.runtimeValidation
      artifact
      publicInput
      proof
      runtimeAccepted
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
  exact And.intro runtimeArtifactEvidence (And.intro verifierAccepts coreObligations)

theorem runtime_pipeline_binding_checked_acceptance_trace_artifact_soundness_obligations
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
        RuntimeTraceConstraintPreflightBindingEvidence
            system
            validation.traceBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeTraceConstraintSoundnessObligations
            system
            validation.traceBindingValidation.traceConstraintValidation
            artifact
            publicInput
            proof
            requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have traceAccepted :=
    runtime_pipeline_binding_checked_acceptance_trace
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_trace_constraint_artifact_binding_checked_acceptance_soundness_obligations
      assumptions
      validation.traceBindingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      traceAccepted

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
  exact
    runtime_pipeline_binding_checked_acceptance_runtime_artifact_soundness_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted

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

theorem runtime_pipeline_binding_checked_acceptance_core_trace_semantic_sound_contract
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
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ RuntimeTraceConstraintSemanticEvidenceComplete
            system
            validation.traceBindingValidation.traceConstraintValidation
            artifact
            publicInput
            proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have verifierSound :=
    runtime_pipeline_binding_checked_acceptance_verifier_sound_witness
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
  have traceSemantic :=
    runtime_pipeline_binding_checked_acceptance_trace_semantic_evidence_complete
      validation
      artifact
      publicInput
      proof
      accepted
  have pcsAndFri :=
    runtime_pipeline_binding_checked_acceptance_pcs_and_fri
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro verifierSound.left
      (And.intro coreObligations
        (And.intro traceSemantic
          (And.intro pcsAndFri.left
            (And.intro pcsAndFri.right verifierSound.right))))

theorem runtime_pipeline_binding_checked_acceptance_runtime_artifact_evidence
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeArtifactEvidence
          system
          validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
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
    runtime_artifact_checked_acceptance_evidence
      proofArtifactValidation.runtimeValidation
      artifact
      publicInput
      proof
      runtimeAccepted

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

theorem runtime_pipeline_binding_checked_acceptance_runtime_soundness_accepts_contract
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
        system.accepts publicInput proof
          /\ RuntimeSoundnessEvidence
            system
            (runtime_pipeline_runtime_soundness_validation validation)
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have runtimeContract :=
    runtime_pipeline_binding_checked_acceptance_runtime_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact And.intro verifierAccepts runtimeContract

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
          /\ SoundWitness system publicInput proof
          /\ RuntimeFriFoldTraceIdentityContract
            system
            validation.queryPlanBindingValidation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeFriFoldQueryPlanOrderContract
            system
            validation.queryPlanBindingValidation.openingValidation
            artifact
            publicInput
            proof := by
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
  have foldTraceIdentityContract :=
    runtime_pipeline_binding_checked_acceptance_fri_fold_trace_identity_contract
      validation
      artifact
      publicInput
      proof
      accepted
  have foldQueryPlanOrderContract :=
    runtime_pipeline_binding_checked_acceptance_fri_fold_query_plan_order_contract
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
      sound.right,
      foldTraceIdentityContract,
      foldQueryPlanOrderContract⟩

set_option linter.style.longLine false in
theorem runtime_pipeline_binding_checked_acceptance_full_soundness_with_fri_parser_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system)
    (boundary :
      RuntimeFriOpeningSegmentParserBoundary
        system
        validation.queryPlanBindingValidation.openingValidation) :
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
          /\ SoundWitness system publicInput proof
          /\ RuntimeFriOpeningSegmentParserContract
            boundary
            artifact
            publicInput
            proof
          /\ RuntimeFriFoldTraceIdentityContract
            system
            validation.queryPlanBindingValidation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeFriFoldQueryPlanOrderContract
            system
            validation.queryPlanBindingValidation.openingValidation
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have fullContract :=
    runtime_pipeline_binding_checked_acceptance_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have parserContract :=
    runtime_pipeline_binding_checked_acceptance_fri_parser_contract
      validation
      boundary
      artifact
      publicInput
      proof
      accepted
  rcases fullContract with
    ⟨pipelineEvidence,
      artifactObligations,
      coreContract,
      executionObligations,
      soundWitness,
      foldTraceIdentityContract,
      foldQueryPlanOrderContract⟩
  exact
    And.intro pipelineEvidence
      (And.intro artifactObligations
        (And.intro coreContract
          (And.intro executionObligations
            (And.intro soundWitness
              (And.intro parserContract
                (And.intro foldTraceIdentityContract foldQueryPlanOrderContract))))))


end Lzvm

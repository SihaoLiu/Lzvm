/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.PipelineBinding.Core
import Lzvm.DigestPrefix

/-!
Runtime proof pipeline binding contracts.
-/

namespace Lzvm

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
  rcases evidence with
    ⟨_, _, _, _, _, _, _, _, openingEvidence, _, _, _, _⟩
  exact openingEvidence.left

theorem runtime_pipeline_binding_evidence_implies_runtime_artifact_evidence
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
        proof := by
  intro evidence
  rcases evidence with
    ⟨_, _, runtimeArtifactEvidence, _, _, _, _, _, _, _, _, _, _⟩
  exact runtimeArtifactEvidence

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

theorem runtime_pipeline_binding_required_external_source_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
          ExternalSourceOpeningEvidence
              system
              (runtime_pipeline_trace_source_validation validation)
              publicInput
              proof
            /\ ExternalSourceOpeningEvidence
              system
              (runtime_pipeline_opening_source_validation validation)
              publicInput
              proof
            /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have requiredSound :=
    runtime_pipeline_binding_required_external_source_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  cases requiredSound with
  | intro _pipelineEvidence tail =>
    cases tail with
    | intro traceExternalEvidence tail =>
      cases tail with
      | intro openingExternalEvidence tail =>
        cases tail with
        | intro _pcsOpeningsValid soundWitness =>
          exact
            And.intro traceExternalEvidence
              (And.intro openingExternalEvidence
                (sound_witness_implies_verifier_core_contract soundWitness))

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
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have transcriptBound :=
    runtime_pipeline_binding_checked_acceptance_transcript_bound_without_assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have publicInputBound :=
    assumptions.semantic.public_input_binding publicInput proof verifierAccepts
  have pcsAndFri :=
    runtime_pipeline_binding_checked_acceptance_pcs_and_fri_without_assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro transcriptBound
      (And.intro publicInputBound pcsAndFri)

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

theorem runtime_pipeline_binding_checked_acceptance_seeded_query_plan_contract
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingSeededContract
          system
          validation.queryPlanBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have queryAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_checked_acceptance_seeded_contract
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryAccepted

theorem runtime_pipeline_binding_checked_acceptance_seed_binds_witness_tree_digests
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have seeded :=
    runtime_pipeline_binding_checked_acceptance_seeded_query_plan_contract
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_seeded_contract_implies_seed_binds_witness_tree_digests
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      seeded

theorem runtime_pipeline_binding_checked_acceptance_seeded_fri_opening_requirements_checked
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have seeded :=
    runtime_pipeline_binding_checked_acceptance_seeded_query_plan_contract
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_seeded_contract_implies_seeded_fri_opening_requirements_checked
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      seeded

theorem runtime_pipeline_binding_checked_acceptance_challenge_transcript_payload_contract
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeChallengeSegmentBindingEvidence
            system
            validation.queryPlanBindingValidation.challengeValidation
            artifact
            publicInput
            proof
          /\ RuntimeTranscriptBindingEvidence
            system
            (runtime_pipeline_transcript_validation validation)
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            (runtime_pipeline_transcript_runtime_validation validation)
            artifact
            publicInput
            proof
          /\ RuntimeTranscriptBindingPayloadContract
            system
            (runtime_pipeline_transcript_validation validation)
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof accepted
  have queryAccepted :=
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
      queryAccepted
  exact
    runtime_challenge_segment_binding_checked_acceptance_transcript_payload_contract
      validation.queryPlanBindingValidation.challengeValidation
      artifact
      publicInput
      proof
      challengeAccepted

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
            have queryPlanBound :=
              runtime_query_plan_binding_evidence_implies_bound_contract
                validation.queryPlanBindingValidation
                artifact
                publicInput
                proof
                queryPlanEvidence
            cases queryPlanBound with
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

theorem runtime_pipeline_binding_checked_acceptance_challenge_query_opening_core_contract
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
        RuntimeChallengeSegmentBindingEvidence
            system
            validation.queryPlanBindingValidation.challengeValidation
            artifact
            publicInput
            proof
          /\ RuntimeTranscriptBindingEvidence
            system
            validation.queryPlanBindingValidation.challengeValidation.transcriptValidation
            artifact
            publicInput
            proof
          /\ RuntimeQueryPlanBindingBoundContract
            system
            validation.queryPlanBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningSegmentBindingBoundContract
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
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have queryAccepted :=
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
      queryAccepted
  have challengeContract :=
    runtime_challenge_segment_binding_checked_acceptance_challenge_and_core_contract
      assumptions
      validation.queryPlanBindingValidation.challengeValidation
      artifact
      publicInput
      proof
      challengeAccepted
  have queryOpeningContract :=
    runtime_query_plan_binding_checked_acceptance_opening_and_core_contract
      assumptions
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      queryAccepted
  rcases challengeContract with
    ⟨challengeEvidence,
      transcriptEvidence,
      _challengeTranscriptBound,
      _challengeCoreObligations⟩
  rcases queryOpeningContract with
    ⟨queryPlanBound,
      openingSegmentBound,
      openingEvidence,
      transcriptBound,
      pcsOpeningsValid,
      friQueriesValid,
      coreObligations⟩
  exact
    And.intro challengeEvidence
      (And.intro transcriptEvidence
        (And.intro queryPlanBound
          (And.intro openingSegmentBound
            (And.intro openingEvidence
              (And.intro transcriptBound
                (And.intro pcsOpeningsValid
                  (And.intro friQueriesValid coreObligations)))))))

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

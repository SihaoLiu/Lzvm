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

theorem runtime_pipeline_binding_evidence_implies_eth_binding_evidence
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
      RuntimeEthBlockPublicInputBindingEvidence
        system
        validation.ethBindingValidation
        artifact
        publicInput
        proof := by
  intro evidence
  rcases evidence with
    ⟨ethEvidence, _, _, _, _, _, _, _, _, _, _, _, _⟩
  exact ethEvidence

theorem runtime_pipeline_binding_evidence_implies_proof_artifact_evidence
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
      RuntimeProofArtifactBindingEvidence
        system
        validation.ethBindingValidation.proofArtifactBindingValidation
        artifact
        publicInput
        proof := by
  intro evidence
  rcases evidence with
    ⟨_, proofArtifactEvidence, _, _, _, _, _, _, _, _, _, _, _⟩
  exact proofArtifactEvidence

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
  have transcriptBound :=
    runtime_pipeline_binding_checked_acceptance_transcript_bound_without_assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have publicInputBound :=
    runtime_pipeline_binding_checked_acceptance_public_input_bound_from_semantic_assumptions
      assumptions.semantic
      validation
      artifact
      publicInput
      proof
      accepted
  have pcsAndFri :=
    runtime_pipeline_binding_checked_acceptance_pcs_and_fri_without_assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    ⟨traceRequired.left,
      openingRequired.right.left,
      And.intro transcriptBound
        (And.intro publicInputBound pcsAndFri)⟩

theorem runtime_pipeline_binding_required_external_source_evidence_core_and_sound
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
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
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
  have coreContract :=
    runtime_pipeline_binding_required_external_source_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  exact
    And.intro requiredSound.left
      (And.intro requiredSound.right.left
        (And.intro requiredSound.right.right.left
          (And.intro requiredSound.right.right.right.left
            (And.intro coreContract.right.right requiredSound.right.right.right.right))))

theorem runtime_pipeline_binding_checked_acceptance_core_obligations_from_semantic_assumptions
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
    semanticAssumptions.public_input_binding publicInput proof verifierAccepts
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
  exact
    runtime_pipeline_binding_checked_acceptance_core_obligations_from_semantic_assumptions
      assumptions.semantic
      validation
      artifact
      publicInput
      proof
      accepted

theorem runtime_pipeline_query_opening_checked_contract_without_assumptions
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
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
          /\ RuntimeOpeningCheckedAcceptance
            system
            validation.queryPlanBindingValidation.openingValidation.openingValidation
            artifact
            publicInput
            proof
          /\ system.transcriptBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  have queryPlanEvidence :=
    runtime_query_plan_binding_checked_acceptance_evidence
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted
  have challengeAccepted :=
    runtime_query_plan_binding_checked_acceptance_challenge
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted
  have challengeEvidence :=
    runtime_challenge_segment_binding_checked_acceptance_evidence
      validation.queryPlanBindingValidation.challengeValidation
      artifact
      publicInput
      proof
      challengeAccepted
  have openingSegmentAccepted :=
    runtime_query_plan_binding_checked_acceptance_opening
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanAccepted
  have openingSegmentEvidence :=
    runtime_query_plan_binding_checked_acceptance_opening_segment_evidence
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
  have transcriptBound :=
    runtime_pipeline_binding_checked_acceptance_transcript_bound_without_assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have pcsAndFri :=
    runtime_pipeline_binding_checked_acceptance_pcs_and_fri_without_assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    ⟨queryPlanEvidence,
      challengeEvidence,
      openingSegmentEvidence,
      openingAccepted,
      transcriptBound,
      pcsAndFri.left,
      pcsAndFri.right⟩

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
  have checkedContract :=
    runtime_pipeline_query_opening_checked_contract_without_assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  rcases checkedContract with
    ⟨queryPlanEvidence,
      challengeEvidence,
      openingSegmentEvidence,
      openingAccepted,
      transcriptBound,
      pcsOpeningsValid,
      friQueriesValid⟩
  have openingEvidence :=
    runtime_opening_checked_acceptance_evidence
      assumptions
      validation.queryPlanBindingValidation.openingValidation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
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

theorem runtime_pipeline_binding_checked_acceptance_challenge_payload_reuse_contract
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeChallengeSegmentPayloadReuseContract
          system
          validation.queryPlanBindingValidation.challengeValidation
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
    runtime_challenge_segment_binding_checked_acceptance_payload_reuse_contract
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
              | intro _transcriptInputsCanonical tail =>
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


end Lzvm

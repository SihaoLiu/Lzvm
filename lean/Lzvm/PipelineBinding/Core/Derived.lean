/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.PipelineBinding.Core.Base

/-!
Derived runtime proof pipeline binding obligations.
-/

namespace Lzvm

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

theorem runtime_pipeline_binding_checked_acceptance_audited_soundness_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimePipelineBindingCheckedAcceptance system validation artifact publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimePipelineBindingEvidence
            system validation artifact publicInput proof requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have audited :=
    runtime_pipeline_binding_checked_acceptance_audited_assumptions
      assumptions validation artifact publicInput proof requiresExternalSource accepted
  exact
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (And.intro
        (assumption_bundle_carries_required_semantic_evidence assumptions)
        audited.right)

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

theorem runtime_pipeline_binding_checked_acceptance_public_input_bound_without_assumptions
    {system : VerifierModel}
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
  have proofArtifactEvidence :=
    runtime_pipeline_binding_checked_acceptance_proof_artifact_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have runtimeArtifactEvidence :=
    runtime_proof_artifact_binding_evidence_implies_runtime_evidence
      validation.ethBindingValidation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      proofArtifactEvidence
  exact
    runtime_artifact_evidence_implies_public_input_bound
      validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
      artifact
      publicInput
      proof
      runtimeArtifactEvidence

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

/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.ProofArtifactBinding

/-!
Runtime Fiat-Shamir transcript binding obligations.
-/

namespace Lzvm

structure RuntimeTranscriptBindingValidation (system : VerifierModel) where
  artifactBindingValidation : RuntimeProofArtifactBindingValidation system
  transcriptBindingAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  challengeSegmentBound : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryPlanBound : RuntimeArtifact -> PublicInput -> Proof -> Prop
  transcriptPayloadMatchesProof : RuntimeArtifact -> PublicInput -> Proof -> Prop
  transcriptAcceptedImpliesArtifactBindingAccepted :
    forall artifact publicInput proof,
      transcriptBindingAccepted artifact publicInput proof ->
        artifactBindingValidation.artifactBindingAccepted artifact publicInput proof
  transcriptAcceptedImpliesChallengeSegmentBound :
    forall artifact publicInput proof,
      transcriptBindingAccepted artifact publicInput proof ->
        challengeSegmentBound artifact publicInput proof
  transcriptAcceptedImpliesQueryPlanBound :
    forall artifact publicInput proof,
      transcriptBindingAccepted artifact publicInput proof ->
        queryPlanBound artifact publicInput proof
  transcriptAcceptedImpliesPayloadMatchesProof :
    forall artifact publicInput proof,
      transcriptBindingAccepted artifact publicInput proof ->
        transcriptPayloadMatchesProof artifact publicInput proof
  transcriptChecksImplyTranscriptBound :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingEvidence
          system
          artifactBindingValidation
          artifact
          publicInput
          proof ->
        challengeSegmentBound artifact publicInput proof ->
          queryPlanBound artifact publicInput proof ->
            transcriptPayloadMatchesProof artifact publicInput proof ->
              system.transcriptBound publicInput proof

def RuntimeTranscriptBindingEvidence
    (system : VerifierModel)
    (validation : RuntimeTranscriptBindingValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeProofArtifactBindingEvidence
      system
      validation.artifactBindingValidation
      artifact
      publicInput
      proof
    /\ validation.challengeSegmentBound artifact publicInput proof
    /\ validation.queryPlanBound artifact publicInput proof
    /\ validation.transcriptPayloadMatchesProof artifact publicInput proof

def RuntimeTranscriptBindingPayloadContract
    (system : VerifierModel)
    (validation : RuntimeTranscriptBindingValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.challengeSegmentBound artifact publicInput proof
    /\ validation.queryPlanBound artifact publicInput proof
    /\ validation.transcriptPayloadMatchesProof artifact publicInput proof
    /\ system.transcriptBound publicInput proof

def RuntimeTranscriptBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeTranscriptBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.transcriptBindingAccepted artifact publicInput proof

theorem runtime_transcript_binding_checked_acceptance_evidence
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTranscriptBindingEvidence
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have artifactAccepted :=
    validation.transcriptAcceptedImpliesArtifactBindingAccepted
      artifact
      publicInput
      proof
      accepted
  have artifactEvidence :=
    runtime_proof_artifact_binding_checked_acceptance_evidence
      validation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  exact
    And.intro artifactEvidence
      (And.intro
        (validation.transcriptAcceptedImpliesChallengeSegmentBound
          artifact
          publicInput
          proof
          accepted)
        (And.intro
          (validation.transcriptAcceptedImpliesQueryPlanBound
            artifact
            publicInput
            proof
            accepted)
          (validation.transcriptAcceptedImpliesPayloadMatchesProof
            artifact
            publicInput
            proof
            accepted)))

theorem runtime_transcript_binding_evidence_implies_transcript_bound
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        system.transcriptBound publicInput proof := by
  intro artifact publicInput proof evidence
  exact
    validation.transcriptChecksImplyTranscriptBound
      artifact
      publicInput
      proof
      evidence.left
      evidence.right.left
      evidence.right.right.left
      evidence.right.right.right

theorem runtime_transcript_binding_evidence_implies_payload_contract
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTranscriptBindingPayloadContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof evidence
  have transcriptBound :=
    runtime_transcript_binding_evidence_implies_transcript_bound
      validation
      artifact
      publicInput
      proof
      evidence
  exact
    And.intro evidence.right.left
      (And.intro evidence.right.right.left
        (And.intro evidence.right.right.right transcriptBound))

theorem runtime_transcript_binding_checked_acceptance_transcript_bound
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.transcriptBound publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_transcript_binding_evidence_implies_transcript_bound
      validation
      artifact
      publicInput
      proof
      (runtime_transcript_binding_checked_acceptance_evidence
        validation
        artifact
        publicInput
        proof
        accepted)

theorem runtime_transcript_binding_checked_acceptance_payload_contract
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTranscriptBindingPayloadContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_transcript_binding_evidence_implies_payload_contract
      validation
      artifact
      publicInput
      proof
      (runtime_transcript_binding_checked_acceptance_evidence
        validation
        artifact
        publicInput
        proof
        accepted)

theorem runtime_transcript_binding_checked_acceptance_artifact_payload_contract
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTranscriptBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.artifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ RuntimeTranscriptBindingPayloadContract
            system
            validation
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof accepted
  have transcriptEvidence :=
    runtime_transcript_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAccepted :=
    validation.transcriptAcceptedImpliesArtifactBindingAccepted
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro transcriptEvidence
      (And.intro
        (runtime_proof_artifact_binding_checked_acceptance_runtime_evidence
          validation.artifactBindingValidation
          artifact
          publicInput
          proof
          artifactAccepted)
        (runtime_transcript_binding_evidence_implies_payload_contract
          validation
          artifact
          publicInput
          proof
          transcriptEvidence))

theorem runtime_transcript_binding_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTranscriptBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.artifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ system.transcriptBound publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have transcriptEvidence :=
    runtime_transcript_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAccepted :=
    validation.transcriptAcceptedImpliesArtifactBindingAccepted
      artifact
      publicInput
      proof
      accepted
  have artifactRuntimeEvidence :=
    runtime_proof_artifact_binding_checked_acceptance_runtime_evidence
      validation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have runtimeAccepted :=
    validation.artifactBindingValidation.bindingAcceptedImpliesRuntimeAccepted
      artifact
      publicInput
      proof
      artifactAccepted
  have verifierAccepts :=
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      validation.artifactBindingValidation.runtimeValidation
      artifact
      publicInput
      proof
      runtimeAccepted
  have transcriptBound :=
    runtime_transcript_binding_evidence_implies_transcript_bound
      validation
      artifact
      publicInput
      proof
      transcriptEvidence
  exact
    And.intro transcriptEvidence
      (And.intro artifactRuntimeEvidence
        (And.intro transcriptBound
          (abstract_verifier_sound assumptions publicInput proof verifierAccepts)))

theorem runtime_transcript_binding_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  have artifactAccepted :=
    validation.transcriptAcceptedImpliesArtifactBindingAccepted
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_proof_artifact_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted

theorem runtime_transcript_binding_checked_acceptance_transcript_and_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTranscriptBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.artifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ system.transcriptBound publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  have transcriptEvidence :=
    runtime_transcript_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAccepted :=
    validation.transcriptAcceptedImpliesArtifactBindingAccepted
      artifact
      publicInput
      proof
      accepted
  have artifactRuntimeEvidence :=
    runtime_proof_artifact_binding_checked_acceptance_runtime_evidence
      validation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have transcriptBound :=
    runtime_transcript_binding_evidence_implies_transcript_bound
      validation
      artifact
      publicInput
      proof
      transcriptEvidence
  have coreContract :=
    runtime_transcript_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro transcriptEvidence
      (And.intro artifactRuntimeEvidence
        (And.intro transcriptBound coreContract))

end Lzvm

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
  transcriptExtensionPayloadOrderCanonical : RuntimeArtifact -> PublicInput -> Proof -> Prop
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
  transcriptAcceptedImpliesExtensionPayloadOrderCanonical :
    forall artifact publicInput proof,
      transcriptBindingAccepted artifact publicInput proof ->
        transcriptExtensionPayloadOrderCanonical artifact publicInput proof
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
              transcriptExtensionPayloadOrderCanonical artifact publicInput proof ->
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
    /\ validation.transcriptExtensionPayloadOrderCanonical artifact publicInput proof

def RuntimeTranscriptBindingPayloadContract
    (system : VerifierModel)
    (validation : RuntimeTranscriptBindingValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.challengeSegmentBound artifact publicInput proof
    /\ validation.queryPlanBound artifact publicInput proof
    /\ validation.transcriptPayloadMatchesProof artifact publicInput proof
    /\ validation.transcriptExtensionPayloadOrderCanonical artifact publicInput proof
    /\ system.transcriptBound publicInput proof

def RuntimeTranscriptBindingStructuralObligations
    (system : VerifierModel)
    (validation : RuntimeTranscriptBindingValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeTranscriptBindingPayloadContract
    system
    validation
    artifact
    publicInput
    proof

def RuntimeTranscriptBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeTranscriptBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.transcriptBindingAccepted artifact publicInput proof

theorem runtime_transcript_binding_checked_acceptance_artifact_finalized
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactFinalized
          system
          validation.artifactBindingValidation
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
  exact
    runtime_proof_artifact_finalized_from_checked_acceptance
      validation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted

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
          (And.intro
            (validation.transcriptAcceptedImpliesPayloadMatchesProof
              artifact
              publicInput
              proof
              accepted)
            (validation.transcriptAcceptedImpliesExtensionPayloadOrderCanonical
              artifact
              publicInput
              proof
              accepted))))

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
      evidence.right.right.right.left
      evidence.right.right.right.right

theorem runtime_transcript_binding_checked_acceptance_extension_payload_order_canonical
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.transcriptExtensionPayloadOrderCanonical
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.transcriptAcceptedImpliesExtensionPayloadOrderCanonical
      artifact
      publicInput
      proof
      accepted

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
        (And.intro evidence.right.right.right.left
          (And.intro evidence.right.right.right.right transcriptBound)))

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

theorem runtime_transcript_binding_checked_acceptance_structural_obligations
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTranscriptBindingStructuralObligations
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_transcript_binding_checked_acceptance_payload_contract
      validation
      artifact
      publicInput
      proof
      accepted

theorem runtime_transcript_binding_checked_acceptance_segment_ids_unique
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.artifactBindingValidation.proofSegmentIdsUnique
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
  exact
    runtime_proof_artifact_binding_checked_acceptance_segment_ids_unique
      validation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted

theorem runtime_transcript_binding_checked_acceptance_unit_values_trace_identity_coverage
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.artifactBindingValidation.proofUnitValuesTraceIdentityCoverage
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
  exact
    runtime_proof_artifact_binding_checked_acceptance_unit_values_trace_identity_coverage
      validation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted

theorem runtime_transcript_binding_checked_acceptance_container_canonical
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.artifactBindingValidation.proofContainerCanonical
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
  exact
    runtime_proof_artifact_binding_checked_acceptance_container_canonical
      validation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted

theorem runtime_transcript_binding_checked_acceptance_metadata_canonical
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.artifactBindingValidation.proofMetadataCanonical
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
  exact
    runtime_proof_artifact_binding_checked_acceptance_metadata_canonical
      validation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted

theorem runtime_transcript_binding_checked_acceptance_segment_payloads_nonempty
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.artifactBindingValidation.proofSegmentPayloadsNonempty
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
  exact
    runtime_proof_artifact_binding_checked_acceptance_segment_payloads_nonempty
      validation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted

theorem runtime_transcript_binding_checked_acceptance_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.artifactBindingValidation.proofSegmentIdsAllowed
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
  exact
    runtime_proof_artifact_binding_checked_acceptance_segment_ids_allowed
      validation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted

theorem runtime_transcript_binding_checked_acceptance_concrete_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.artifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  have artifactAccepted :=
    validation.transcriptAcceptedImpliesArtifactBindingAccepted
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_proof_artifact_binding_checked_acceptance_concrete_segment_ids_allowed
      validation.artifactBindingValidation
      binding
      artifact
      publicInput
      proof
      artifactAccepted

theorem runtime_transcript_binding_checked_acceptance_segments_present
    {system : VerifierModel}
    (validation : RuntimeTranscriptBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.artifactBindingValidation.proofSegmentsPresent
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
  exact
    runtime_proof_artifact_binding_checked_acceptance_segments_present
      validation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted

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
  have artifactFinalized :=
    runtime_transcript_binding_checked_acceptance_artifact_finalized
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactFull :=
    runtime_proof_artifact_finalized_full_contract
      assumptions
      validation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactFinalized
  have transcriptBound :=
    runtime_transcript_binding_evidence_implies_transcript_bound
      validation
      artifact
      publicInput
      proof
      transcriptEvidence
  exact
    And.intro transcriptEvidence
      (And.intro artifactFull.right.right.left
        (And.intro transcriptBound artifactFull.right.right.right))

theorem runtime_transcript_binding_checked_acceptance_full_contract
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
          /\ RuntimeTranscriptBindingStructuralObligations
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
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_transcript_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have structural :=
    runtime_transcript_binding_checked_acceptance_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro sound.left
      (And.intro structural
        (And.intro sound.right.left sound.right.right.right))

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
  have artifactFinalized :=
    runtime_transcript_binding_checked_acceptance_artifact_finalized
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_proof_artifact_finalized_verifier_core_contract
      assumptions
      validation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactFinalized

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

theorem runtime_transcript_binding_checked_acceptance_evidence_core_and_sound
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
          /\ RuntimeTranscriptBindingStructuralObligations
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
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have fullContract :=
    runtime_transcript_binding_checked_acceptance_full_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have transcriptBound :=
    runtime_transcript_binding_evidence_implies_transcript_bound
      validation
      artifact
      publicInput
      proof
      fullContract.left
  have coreContract :=
    runtime_transcript_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro fullContract.left
      (And.intro fullContract.right.left
        (And.intro fullContract.right.right.left
          (And.intro transcriptBound
            (And.intro coreContract fullContract.right.right.right))))

theorem
  runtime_transcript_binding_checked_acceptance_concrete_core_sound_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeTranscriptBindingValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.artifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimeTranscriptBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        (RuntimeTranscriptBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeTranscriptBindingStructuralObligations
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
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof)
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (runtime_transcript_binding_checked_acceptance_evidence_core_and_sound
        assumptions
        validation
        artifact
        publicInput
        proof
        accepted)
      (runtime_transcript_binding_checked_acceptance_concrete_segment_ids_allowed
        validation
        binding
        artifact
        publicInput
        proof
        accepted)

end Lzvm

/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.TranscriptBinding

/-!
Runtime challenge segment binding obligations.
-/

namespace Lzvm

structure RuntimeChallengeSegmentBindingValidation (system : VerifierModel) where
  transcriptValidation : RuntimeTranscriptBindingValidation system
  challengeBindingAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  challengeSegmentPayloadValid : RuntimeArtifact -> PublicInput -> Proof -> Prop
  challengeSegmentMatchesTranscript : RuntimeArtifact -> PublicInput -> Proof -> Prop
  challengeBindingAcceptedImpliesTranscriptAccepted :
    forall artifact publicInput proof,
      challengeBindingAccepted artifact publicInput proof ->
        transcriptValidation.transcriptBindingAccepted artifact publicInput proof
  challengeBindingAcceptedImpliesPayloadValid :
    forall artifact publicInput proof,
      challengeBindingAccepted artifact publicInput proof ->
        challengeSegmentPayloadValid artifact publicInput proof
  challengeBindingAcceptedImpliesSegmentMatchesTranscript :
    forall artifact publicInput proof,
      challengeBindingAccepted artifact publicInput proof ->
        challengeSegmentMatchesTranscript artifact publicInput proof
  challengeSegmentChecksImplyBound :
    forall artifact publicInput proof,
      challengeSegmentPayloadValid artifact publicInput proof ->
        challengeSegmentMatchesTranscript artifact publicInput proof ->
          transcriptValidation.challengeSegmentBound artifact publicInput proof

def RuntimeChallengeSegmentBindingEvidence
    (_system : VerifierModel)
    (validation : RuntimeChallengeSegmentBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.challengeSegmentPayloadValid artifact publicInput proof
    /\ validation.challengeSegmentMatchesTranscript artifact publicInput proof
    /\ validation.transcriptValidation.challengeSegmentBound artifact publicInput proof

def RuntimeChallengeSegmentBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeChallengeSegmentBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.challengeBindingAccepted artifact publicInput proof

theorem runtime_challenge_segment_binding_checked_acceptance_evidence
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeChallengeSegmentBindingEvidence
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have payloadValid :=
    validation.challengeBindingAcceptedImpliesPayloadValid
      artifact
      publicInput
      proof
      accepted
  have segmentMatchesTranscript :=
    validation.challengeBindingAcceptedImpliesSegmentMatchesTranscript
      artifact
      publicInput
      proof
      accepted
  have challengeSegmentBound :=
    validation.challengeSegmentChecksImplyBound
      artifact
      publicInput
      proof
      payloadValid
      segmentMatchesTranscript
  exact
    And.intro payloadValid
      (And.intro segmentMatchesTranscript challengeSegmentBound)

theorem runtime_challenge_segment_binding_evidence_implies_payload_valid
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        validation.challengeSegmentPayloadValid artifact publicInput proof := by
  intro artifact publicInput proof evidence
  exact evidence.left

theorem runtime_challenge_segment_binding_evidence_implies_segment_matches_transcript
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        validation.challengeSegmentMatchesTranscript artifact publicInput proof := by
  intro artifact publicInput proof evidence
  exact evidence.right.left

theorem runtime_challenge_segment_binding_evidence_implies_challenge_segment_bound
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        validation.transcriptValidation.challengeSegmentBound artifact publicInput proof := by
  intro artifact publicInput proof evidence
  exact evidence.right.right

theorem runtime_challenge_segment_binding_checked_acceptance_transcript
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTranscriptBindingCheckedAcceptance
          system
          validation.transcriptValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.challengeBindingAcceptedImpliesTranscriptAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_challenge_segment_binding_checked_acceptance_payload_valid
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.challengeSegmentPayloadValid artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.challengeBindingAcceptedImpliesPayloadValid
      artifact
      publicInput
      proof
      accepted

theorem runtime_challenge_segment_binding_checked_acceptance_segment_matches_transcript
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.challengeSegmentMatchesTranscript artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.challengeBindingAcceptedImpliesSegmentMatchesTranscript
      artifact
      publicInput
      proof
      accepted

theorem runtime_challenge_segment_binding_checked_acceptance_challenge_segment_bound
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.transcriptValidation.challengeSegmentBound artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have payloadValid :=
    validation.challengeBindingAcceptedImpliesPayloadValid
      artifact
      publicInput
      proof
      accepted
  have segmentMatchesTranscript :=
    validation.challengeBindingAcceptedImpliesSegmentMatchesTranscript
      artifact
      publicInput
      proof
      accepted
  exact
    validation.challengeSegmentChecksImplyBound
      artifact
      publicInput
      proof
      payloadValid
      segmentMatchesTranscript

theorem runtime_challenge_segment_binding_checked_acceptance_transcript_payload_contract
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeChallengeSegmentBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeTranscriptBindingEvidence
            system
            validation.transcriptValidation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.transcriptValidation.artifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ RuntimeTranscriptBindingPayloadContract
            system
            validation.transcriptValidation
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof accepted
  have challengeEvidence :=
    runtime_challenge_segment_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have transcriptAccepted :=
    runtime_challenge_segment_binding_checked_acceptance_transcript
      validation
      artifact
      publicInput
      proof
      accepted
  have transcriptContracts :=
    runtime_transcript_binding_checked_acceptance_artifact_payload_contract
      validation.transcriptValidation
      artifact
      publicInput
      proof
      transcriptAccepted
  exact
    And.intro challengeEvidence
      (And.intro transcriptContracts.left
        (And.intro
          transcriptContracts.right.left
          transcriptContracts.right.right))

theorem runtime_challenge_segment_binding_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeChallengeSegmentBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeTranscriptBindingEvidence
            system
            validation.transcriptValidation
            artifact
            publicInput
            proof
          /\ system.transcriptBound publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have challengeEvidence :=
    runtime_challenge_segment_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have transcriptAccepted :=
    runtime_challenge_segment_binding_checked_acceptance_transcript
      validation
      artifact
      publicInput
      proof
      accepted
  have transcriptSound :=
    runtime_transcript_binding_checked_acceptance_sound
      assumptions
      validation.transcriptValidation
      artifact
      publicInput
      proof
      transcriptAccepted
  exact
    And.intro challengeEvidence
      (And.intro transcriptSound.left
        (And.intro transcriptSound.right.right.left transcriptSound.right.right.right))

theorem runtime_challenge_segment_binding_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_challenge_segment_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact sound_witness_implies_verifier_core_contract sound.right.right.right

theorem runtime_challenge_segment_binding_checked_acceptance_challenge_and_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeChallengeSegmentBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeTranscriptBindingEvidence
            system
            validation.transcriptValidation
            artifact
            publicInput
            proof
          /\ system.transcriptBound publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_challenge_segment_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro sound.left
      (And.intro sound.right.left
        (And.intro sound.right.right.left
          (sound_witness_implies_verifier_core_contract
            sound.right.right.right)))

end Lzvm

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

end Lzvm

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
  challengeQueryNonceValid : RuntimeArtifact -> PublicInput -> Proof -> Prop
  challengeQueriesDerivedFromNonce : RuntimeArtifact -> PublicInput -> Proof -> Prop
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
  challengeBindingAcceptedImpliesQueryNonceValid :
    forall artifact publicInput proof,
      challengeBindingAccepted artifact publicInput proof ->
        challengeQueryNonceValid artifact publicInput proof
  challengeBindingAcceptedImpliesQueriesDerivedFromNonce :
    forall artifact publicInput proof,
      challengeBindingAccepted artifact publicInput proof ->
        challengeQueriesDerivedFromNonce artifact publicInput proof
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

def RuntimeChallengeQueryDerivationContract
    (_system : VerifierModel)
    (validation : RuntimeChallengeSegmentBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.challengeQueryNonceValid artifact publicInput proof
    /\ validation.challengeQueriesDerivedFromNonce artifact publicInput proof
    /\ validation.transcriptValidation.queryPlanBound artifact publicInput proof

def RuntimeChallengeSegmentPayloadReuseContract
    (_system : VerifierModel)
    (validation : RuntimeChallengeSegmentBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  let artifactValidation := validation.transcriptValidation.artifactBindingValidation
  validation.challengeSegmentPayloadValid artifact publicInput proof
    /\ validation.challengeSegmentMatchesTranscript artifact publicInput proof
    /\ validation.transcriptValidation.challengeSegmentBound artifact publicInput proof
    /\ artifactValidation.proofSegmentIdsUnique artifact publicInput proof
    /\ artifactValidation.proofUnitValuesTraceIdentityCoverage artifact publicInput proof

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

theorem runtime_challenge_segment_binding_checked_acceptance_artifact_finalized
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactFinalized
          system
          validation.transcriptValidation.artifactBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have transcriptAccepted :=
    runtime_challenge_segment_binding_checked_acceptance_transcript
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_transcript_binding_checked_acceptance_artifact_finalized
      validation.transcriptValidation
      artifact
      publicInput
      proof
      transcriptAccepted

theorem runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactBindingStructuralObligations
          system
          validation.transcriptValidation.artifactBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have artifactFinalized :=
    runtime_challenge_segment_binding_checked_acceptance_artifact_finalized
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_proof_artifact_finalized_structural_obligations
      validation.transcriptValidation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactFinalized

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

theorem runtime_challenge_segment_binding_checked_acceptance_segment_ids_unique
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.transcriptValidation.artifactBindingValidation.proofSegmentIdsUnique
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have artifactStructural :=
    runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.right.right.right.right.left

theorem runtime_challenge_segment_binding_checked_acceptance_unit_values_trace_identity_coverage
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let artifactValidation := validation.transcriptValidation.artifactBindingValidation
        artifactValidation.proofUnitValuesTraceIdentityCoverage artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have artifactStructural :=
    runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.right.right.right.right.right

theorem runtime_challenge_segment_binding_checked_acceptance_payload_reuse_contract
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeChallengeSegmentPayloadReuseContract
          system
          validation
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
  have segmentIdsUnique :=
    runtime_challenge_segment_binding_checked_acceptance_segment_ids_unique
      validation
      artifact
      publicInput
      proof
      accepted
  have unitValuesTraceIdentityCoverage :=
    runtime_challenge_segment_binding_checked_acceptance_unit_values_trace_identity_coverage
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro challengeEvidence.left
      (And.intro challengeEvidence.right.left
        (And.intro challengeEvidence.right.right
          (And.intro segmentIdsUnique unitValuesTraceIdentityCoverage)))

theorem runtime_challenge_segment_binding_checked_acceptance_query_derivation_contract
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeChallengeQueryDerivationContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have transcriptAccepted :=
    validation.challengeBindingAcceptedImpliesTranscriptAccepted
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro
      (validation.challengeBindingAcceptedImpliesQueryNonceValid
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (validation.challengeBindingAcceptedImpliesQueriesDerivedFromNonce
          artifact
          publicInput
          proof
          accepted)
        (validation.transcriptValidation.transcriptAcceptedImpliesQueryPlanBound
          artifact
          publicInput
          proof
          transcriptAccepted))

theorem runtime_challenge_segment_binding_checked_acceptance_container_canonical
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.transcriptValidation.artifactBindingValidation.proofContainerCanonical
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have artifactStructural :=
    runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.left

theorem runtime_challenge_segment_binding_checked_acceptance_metadata_canonical
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.transcriptValidation.artifactBindingValidation.proofMetadataCanonical
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have artifactStructural :=
    runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.left

theorem runtime_challenge_segment_binding_checked_acceptance_segment_payloads_nonempty
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.transcriptValidation.artifactBindingValidation.proofSegmentPayloadsNonempty
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have artifactStructural :=
    runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.right.right.left

theorem runtime_challenge_segment_binding_checked_acceptance_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.transcriptValidation.artifactBindingValidation.proofSegmentIdsAllowed
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have artifactStructural :=
    runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.right.right.right.left

theorem runtime_challenge_segment_binding_checked_acceptance_concrete_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  have transcriptAccepted :=
    validation.challengeBindingAcceptedImpliesTranscriptAccepted
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_transcript_binding_checked_acceptance_concrete_segment_ids_allowed
      validation.transcriptValidation
      binding
      artifact
      publicInput
      proof
      transcriptAccepted

theorem runtime_challenge_segment_binding_checked_acceptance_segments_present
    {system : VerifierModel}
    (validation : RuntimeChallengeSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.transcriptValidation.artifactBindingValidation.proofSegmentsPresent
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have artifactStructural :=
    runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.right.left

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
  have transcriptFull :=
    runtime_transcript_binding_checked_acceptance_full_contract
      assumptions
      validation.transcriptValidation
      artifact
      publicInput
      proof
      transcriptAccepted
  have transcriptBound :=
    runtime_transcript_binding_evidence_implies_transcript_bound
      validation.transcriptValidation
      artifact
      publicInput
      proof
      transcriptFull.left
  exact
    And.intro challengeEvidence
      (And.intro transcriptFull.left
        (And.intro transcriptBound transcriptFull.right.right.right))

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
  have transcriptAccepted :=
    runtime_challenge_segment_binding_checked_acceptance_transcript
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_transcript_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation.transcriptValidation
      artifact
      publicInput
      proof
      transcriptAccepted

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
  have transcriptEvidence :=
    runtime_transcript_binding_checked_acceptance_evidence
      validation.transcriptValidation
      artifact
      publicInput
      proof
      transcriptAccepted
  have transcriptBound :=
    runtime_transcript_binding_evidence_implies_transcript_bound
      validation.transcriptValidation
      artifact
      publicInput
      proof
      transcriptEvidence
  have coreContract :=
    runtime_challenge_segment_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro challengeEvidence
      (And.intro transcriptEvidence
        (And.intro transcriptBound coreContract))

end Lzvm

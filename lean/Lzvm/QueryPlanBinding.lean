/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.ChallengeSegmentBinding
import Lzvm.OpeningSegmentBinding

/-!
Runtime query plan binding obligations.
-/

namespace Lzvm

structure RuntimeQueryPlanBindingValidation (system : VerifierModel) where
  challengeValidation : RuntimeChallengeSegmentBindingValidation system
  openingValidation : RuntimeOpeningSegmentBindingValidation system
  queryPlanBindingAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryPlanSegmentCanonical : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryPlanDerivedFromTranscript : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryPlanMatchesOpenedArtifacts : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryPlanBindingAcceptedImpliesChallengeAccepted :
    forall artifact publicInput proof,
      queryPlanBindingAccepted artifact publicInput proof ->
        challengeValidation.challengeBindingAccepted artifact publicInput proof
  queryPlanBindingAcceptedImpliesOpeningAccepted :
    forall artifact publicInput proof,
      queryPlanBindingAccepted artifact publicInput proof ->
        openingValidation.openingSegmentBindingAccepted artifact publicInput proof
  queryPlanBindingAcceptedImpliesSegmentCanonical :
    forall artifact publicInput proof,
      queryPlanBindingAccepted artifact publicInput proof ->
        queryPlanSegmentCanonical artifact publicInput proof
  queryPlanBindingAcceptedImpliesDerivedFromTranscript :
    forall artifact publicInput proof,
      queryPlanBindingAccepted artifact publicInput proof ->
        queryPlanDerivedFromTranscript artifact publicInput proof
  queryPlanBindingAcceptedImpliesMatchesOpenedArtifacts :
    forall artifact publicInput proof,
      queryPlanBindingAccepted artifact publicInput proof ->
        queryPlanMatchesOpenedArtifacts artifact publicInput proof
  queryPlanChecksImplyTranscriptQueryPlanBound :
    forall artifact publicInput proof,
      queryPlanSegmentCanonical artifact publicInput proof ->
        queryPlanDerivedFromTranscript artifact publicInput proof ->
          challengeValidation.transcriptValidation.queryPlanBound artifact publicInput proof
  queryPlanChecksImplyOpeningQueryPlanBound :
    forall artifact publicInput proof,
      queryPlanSegmentCanonical artifact publicInput proof ->
        queryPlanMatchesOpenedArtifacts artifact publicInput proof ->
          openingValidation.queryPlanBound artifact publicInput proof

def RuntimeQueryPlanBindingBoundContract
    (_system : VerifierModel)
    (validation : RuntimeQueryPlanBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.queryPlanSegmentCanonical artifact publicInput proof
    /\ validation.queryPlanDerivedFromTranscript artifact publicInput proof
    /\ validation.queryPlanMatchesOpenedArtifacts artifact publicInput proof
    /\ validation.challengeValidation.transcriptValidation.queryPlanBound
      artifact
      publicInput
      proof
    /\ validation.openingValidation.queryPlanBound artifact publicInput proof

def RuntimeQueryPlanBindingEvidence
    (_system : VerifierModel)
    (validation : RuntimeQueryPlanBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeQueryPlanBindingBoundContract
    _system
    validation
    artifact
    publicInput
    proof

def RuntimeQueryPlanBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeQueryPlanBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.queryPlanBindingAccepted artifact publicInput proof

theorem runtime_query_plan_binding_checked_acceptance_evidence
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingEvidence
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have segmentCanonical :=
    validation.queryPlanBindingAcceptedImpliesSegmentCanonical
      artifact
      publicInput
      proof
      accepted
  have derivedFromTranscript :=
    validation.queryPlanBindingAcceptedImpliesDerivedFromTranscript
      artifact
      publicInput
      proof
      accepted
  have matchesOpenedArtifacts :=
    validation.queryPlanBindingAcceptedImpliesMatchesOpenedArtifacts
      artifact
      publicInput
      proof
      accepted
  have transcriptQueryPlanBound :=
    validation.queryPlanChecksImplyTranscriptQueryPlanBound
      artifact
      publicInput
      proof
      segmentCanonical
      derivedFromTranscript
  have openingQueryPlanBound :=
    validation.queryPlanChecksImplyOpeningQueryPlanBound
      artifact
      publicInput
      proof
      segmentCanonical
      matchesOpenedArtifacts
  exact
    And.intro segmentCanonical
      (And.intro derivedFromTranscript
        (And.intro matchesOpenedArtifacts
          (And.intro transcriptQueryPlanBound openingQueryPlanBound)))

theorem runtime_query_plan_binding_evidence_implies_bound_contract
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingBoundContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof evidence
  exact evidence

theorem runtime_query_plan_binding_checked_acceptance_challenge
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation.challengeValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.queryPlanBindingAcceptedImpliesChallengeAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_query_plan_binding_checked_acceptance_opening
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation.openingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.queryPlanBindingAcceptedImpliesOpeningAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_query_plan_binding_checked_acceptance_bound_contract
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingBoundContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_query_plan_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_evidence_implies_bound_contract
      validation
      artifact
      publicInput
      proof
      evidence

theorem runtime_query_plan_binding_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeChallengeSegmentBindingEvidence
            system
            validation.challengeValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningSegmentBindingEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ system.transcriptBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have queryPlanEvidence :=
    runtime_query_plan_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have challengeAccepted :=
    runtime_query_plan_binding_checked_acceptance_challenge
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAccepted :=
    runtime_query_plan_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  have challengeSound :=
    runtime_challenge_segment_binding_checked_acceptance_sound
      assumptions
      validation.challengeValidation
      artifact
      publicInput
      proof
      challengeAccepted
  have openingSound :=
    runtime_opening_segment_binding_checked_acceptance_sound
      assumptions
      validation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
  have pcsAndFri :=
    runtime_opening_evidence_implies_pcs_and_fri
      validation.openingValidation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingSound.right.left
  exact
    And.intro queryPlanEvidence
      (And.intro challengeSound.left
        (And.intro openingSound.left
          (And.intro openingSound.right.left
            (And.intro challengeSound.right.right.left
              (And.intro pcsAndFri.left
                (And.intro pcsAndFri.right openingSound.right.right))))))

end Lzvm

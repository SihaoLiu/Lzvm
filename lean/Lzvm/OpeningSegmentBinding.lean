/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.OpeningValidation

/-!
Runtime opening segment binding obligations.
-/

namespace Lzvm

structure RuntimeOpeningSegmentBindingValidation (system : VerifierModel) where
  openingValidation : RuntimeOpeningValidation system
  openingSegmentBindingAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryPlanBound : RuntimeArtifact -> PublicInput -> Proof -> Prop
  constantOpeningSegmentsValid : RuntimeArtifact -> PublicInput -> Proof -> Prop
  witnessOpeningSegmentsValid : RuntimeArtifact -> PublicInput -> Proof -> Prop
  friOpeningSegmentsValid : RuntimeArtifact -> PublicInput -> Proof -> Prop
  friFoldsValid : RuntimeArtifact -> PublicInput -> Proof -> Prop
  verifierQueryOutputsValid : RuntimeArtifact -> PublicInput -> Proof -> Prop
  openingSegmentBindingAcceptedImpliesOpeningAccepted :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        openingValidation.openingAccepted artifact publicInput proof
  openingSegmentBindingAcceptedImpliesQueryPlanBound :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        queryPlanBound artifact publicInput proof
  openingSegmentBindingAcceptedImpliesConstantOpeningSegmentsValid :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        constantOpeningSegmentsValid artifact publicInput proof
  openingSegmentBindingAcceptedImpliesWitnessOpeningSegmentsValid :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        witnessOpeningSegmentsValid artifact publicInput proof
  openingSegmentBindingAcceptedImpliesFriOpeningSegmentsValid :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        friOpeningSegmentsValid artifact publicInput proof
  openingSegmentBindingAcceptedImpliesFriFoldsValid :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        friFoldsValid artifact publicInput proof
  openingSegmentBindingAcceptedImpliesVerifierQueryOutputsValid :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        verifierQueryOutputsValid artifact publicInput proof
  openingSegmentChecksImplyConstantOpeningsBound :
    forall artifact publicInput proof,
      queryPlanBound artifact publicInput proof ->
        constantOpeningSegmentsValid artifact publicInput proof ->
          openingValidation.constantOpeningsBound artifact publicInput proof
  openingSegmentChecksImplyWitnessOpeningsBound :
    forall artifact publicInput proof,
      queryPlanBound artifact publicInput proof ->
        witnessOpeningSegmentsValid artifact publicInput proof ->
          openingValidation.witnessOpeningsBound artifact publicInput proof
  openingSegmentChecksImplyFriOpeningBound :
    forall artifact publicInput proof,
      queryPlanBound artifact publicInput proof ->
        friOpeningSegmentsValid artifact publicInput proof ->
          friFoldsValid artifact publicInput proof ->
            verifierQueryOutputsValid artifact publicInput proof ->
              openingValidation.friOpeningBound artifact publicInput proof

def RuntimeOpeningSegmentBindingBoundContract
    (_system : VerifierModel)
    (validation : RuntimeOpeningSegmentBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.queryPlanBound artifact publicInput proof
    /\ validation.constantOpeningSegmentsValid artifact publicInput proof
    /\ validation.witnessOpeningSegmentsValid artifact publicInput proof
    /\ validation.friOpeningSegmentsValid artifact publicInput proof
    /\ validation.friFoldsValid artifact publicInput proof
    /\ validation.verifierQueryOutputsValid artifact publicInput proof
    /\ validation.openingValidation.constantOpeningsBound artifact publicInput proof
    /\ validation.openingValidation.witnessOpeningsBound artifact publicInput proof
    /\ validation.openingValidation.friOpeningBound artifact publicInput proof

def RuntimeOpeningSegmentBindingEvidence
    (_system : VerifierModel)
    (validation : RuntimeOpeningSegmentBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeOpeningSegmentBindingBoundContract
    _system
    validation
    artifact
    publicInput
    proof

def RuntimeOpeningSegmentBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeOpeningSegmentBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.openingSegmentBindingAccepted artifact publicInput proof

theorem runtime_opening_segment_binding_checked_acceptance_evidence
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingEvidence
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have queryPlanBound :=
    validation.openingSegmentBindingAcceptedImpliesQueryPlanBound
      artifact
      publicInput
      proof
      accepted
  have constantSegments :=
    validation.openingSegmentBindingAcceptedImpliesConstantOpeningSegmentsValid
      artifact
      publicInput
      proof
      accepted
  have witnessSegments :=
    validation.openingSegmentBindingAcceptedImpliesWitnessOpeningSegmentsValid
      artifact
      publicInput
      proof
      accepted
  have friSegments :=
    validation.openingSegmentBindingAcceptedImpliesFriOpeningSegmentsValid
      artifact
      publicInput
      proof
      accepted
  have friFolds :=
    validation.openingSegmentBindingAcceptedImpliesFriFoldsValid
      artifact
      publicInput
      proof
      accepted
  have verifierQueries :=
    validation.openingSegmentBindingAcceptedImpliesVerifierQueryOutputsValid
      artifact
      publicInput
      proof
      accepted
  have constantBound :=
    validation.openingSegmentChecksImplyConstantOpeningsBound
      artifact
      publicInput
      proof
      queryPlanBound
      constantSegments
  have witnessBound :=
    validation.openingSegmentChecksImplyWitnessOpeningsBound
      artifact
      publicInput
      proof
      queryPlanBound
      witnessSegments
  have friOpeningBound :=
    validation.openingSegmentChecksImplyFriOpeningBound
      artifact
      publicInput
      proof
      queryPlanBound
      friSegments
      friFolds
      verifierQueries
  exact
    And.intro queryPlanBound
      (And.intro constantSegments
        (And.intro witnessSegments
          (And.intro friSegments
            (And.intro friFolds
              (And.intro verifierQueries
                (And.intro constantBound
                  (And.intro witnessBound friOpeningBound)))))))

theorem runtime_opening_segment_binding_evidence_implies_bound_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingBoundContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof evidence
  exact evidence

theorem runtime_opening_segment_binding_checked_acceptance_opening
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningCheckedAcceptance
          system
          validation.openingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.openingSegmentBindingAcceptedImpliesOpeningAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_opening_segment_binding_checked_acceptance_bound_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingBoundContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_segment_binding_evidence_implies_bound_contract
      validation
      artifact
      publicInput
      proof
      evidence

theorem runtime_opening_segment_binding_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have segmentEvidence :=
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAccepted :=
    runtime_opening_segment_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  have openingSound :=
    runtime_opening_checked_acceptance_sound
      assumptions
      validation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
  exact And.intro segmentEvidence openingSound

theorem runtime_opening_segment_binding_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof _requiresExternalSource accepted
  have sound :=
    runtime_opening_segment_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      _requiresExternalSource
      accepted
  exact sound_witness_implies_verifier_core_contract sound.right.right

end Lzvm

/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.OpeningSegmentBinding

/-!
Runtime obligations for optional constant-opening segments.
-/

namespace Lzvm

structure RuntimeOptionalConstantOpeningValidation (system : VerifierModel) where
  openingSegmentValidation : RuntimeOpeningSegmentBindingValidation system
  optionalConstantOpeningAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  constantOpeningSegmentAbsent : RuntimeArtifact -> PublicInput -> Proof -> Prop
  constantOpeningSegmentPresent : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queriedUnit : RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  queriedUnitRequiresConstants : RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  queriedUnitHasZeroConstantWidth : RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  matchingConstantOpeningUnitPresent :
    RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  verifierConstantValuesEmpty : RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  verifierQueryRowsRecoveredFromWitnessOpening :
    RuntimeArtifact -> PublicInput -> Proof -> Nat -> Prop
  unexpectedConstantOpeningUnitsRejected : RuntimeArtifact -> PublicInput -> Proof -> Prop
  optionalConstantOpeningAcceptedImpliesOpeningSegmentBindingAccepted :
    forall artifact publicInput proof,
      optionalConstantOpeningAccepted artifact publicInput proof ->
        openingSegmentValidation.openingSegmentBindingAccepted
          artifact
          publicInput
          proof
  optionalConstantOpeningAcceptedImpliesConstantOpeningSegmentsValid :
    forall artifact publicInput proof,
      optionalConstantOpeningAccepted artifact publicInput proof ->
        openingSegmentValidation.constantOpeningSegmentsValid artifact publicInput proof
  optionalConstantOpeningAcceptedAndAbsentImpliesQueriedUnitZeroWidth :
    forall artifact publicInput proof unit,
      optionalConstantOpeningAccepted artifact publicInput proof ->
        constantOpeningSegmentAbsent artifact publicInput proof ->
          queriedUnit artifact publicInput proof unit ->
            queriedUnitHasZeroConstantWidth artifact publicInput proof unit
  optionalConstantOpeningAcceptedAndAbsentImpliesNoQueriedUnitRequiresConstants :
    forall artifact publicInput proof unit,
      optionalConstantOpeningAccepted artifact publicInput proof ->
        constantOpeningSegmentAbsent artifact publicInput proof ->
          queriedUnit artifact publicInput proof unit ->
            ¬ queriedUnitRequiresConstants artifact publicInput proof unit
  optionalConstantOpeningAcceptedAndRequiresConstantsImpliesMatchingUnitPresent :
    forall artifact publicInput proof unit,
      optionalConstantOpeningAccepted artifact publicInput proof ->
        queriedUnit artifact publicInput proof unit ->
          queriedUnitRequiresConstants artifact publicInput proof unit ->
            matchingConstantOpeningUnitPresent artifact publicInput proof unit
  optionalConstantOpeningAcceptedAndZeroWidthImpliesVerifierConstantValuesEmpty :
    forall artifact publicInput proof unit,
      optionalConstantOpeningAccepted artifact publicInput proof ->
        queriedUnit artifact publicInput proof unit ->
          queriedUnitHasZeroConstantWidth artifact publicInput proof unit ->
            verifierConstantValuesEmpty artifact publicInput proof unit
  optionalConstantOpeningAcceptedAndZeroWidthImpliesRowsRecoveredFromWitnessOpening :
    forall artifact publicInput proof unit,
      optionalConstantOpeningAccepted artifact publicInput proof ->
        queriedUnit artifact publicInput proof unit ->
          queriedUnitHasZeroConstantWidth artifact publicInput proof unit ->
            verifierQueryRowsRecoveredFromWitnessOpening artifact publicInput proof unit
  optionalConstantOpeningAcceptedAndPresentImpliesUnexpectedUnitsRejected :
    forall artifact publicInput proof,
      optionalConstantOpeningAccepted artifact publicInput proof ->
        constantOpeningSegmentPresent artifact publicInput proof ->
          unexpectedConstantOpeningUnitsRejected artifact publicInput proof

def RuntimeOptionalConstantOpeningCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeOptionalConstantOpeningValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.optionalConstantOpeningAccepted artifact publicInput proof

def RuntimeOptionalConstantOpeningContract
    (_system : VerifierModel)
    (validation : RuntimeOptionalConstantOpeningValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.openingSegmentValidation.constantOpeningSegmentsValid
      artifact
      publicInput
      proof
    /\ validation.openingSegmentValidation.openingValidation.constantOpeningsBound
      artifact
      publicInput
      proof
    /\ (validation.constantOpeningSegmentAbsent artifact publicInput proof ->
      forall unit,
        validation.queriedUnit artifact publicInput proof unit ->
          validation.queriedUnitHasZeroConstantWidth artifact publicInput proof unit
            /\ ¬ validation.queriedUnitRequiresConstants artifact publicInput proof unit)
    /\ (forall unit,
      validation.queriedUnit artifact publicInput proof unit ->
        validation.queriedUnitRequiresConstants artifact publicInput proof unit ->
          validation.matchingConstantOpeningUnitPresent artifact publicInput proof unit)
    /\ (forall unit,
      validation.queriedUnit artifact publicInput proof unit ->
        validation.queriedUnitHasZeroConstantWidth artifact publicInput proof unit ->
          validation.verifierConstantValuesEmpty artifact publicInput proof unit
            /\ validation.verifierQueryRowsRecoveredFromWitnessOpening
              artifact
              publicInput
              proof
              unit)
    /\ (validation.constantOpeningSegmentPresent artifact publicInput proof ->
      validation.unexpectedConstantOpeningUnitsRejected artifact publicInput proof)

def RuntimeOptionalConstantOpeningAbsentSegmentContract
    (_system : VerifierModel)
    (validation : RuntimeOptionalConstantOpeningValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.constantOpeningSegmentAbsent artifact publicInput proof
    /\ validation.openingSegmentValidation.openingValidation.constantOpeningsBound
      artifact
      publicInput
      proof
    /\ forall unit,
      validation.queriedUnit artifact publicInput proof unit ->
        validation.queriedUnitHasZeroConstantWidth artifact publicInput proof unit
          /\ ¬ validation.queriedUnitRequiresConstants artifact publicInput proof unit

def RuntimeOptionalConstantOpeningZeroWidthQueryContract
    (_system : VerifierModel)
    (validation : RuntimeOptionalConstantOpeningValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  forall unit,
    validation.queriedUnit artifact publicInput proof unit ->
      validation.queriedUnitHasZeroConstantWidth artifact publicInput proof unit ->
        validation.verifierConstantValuesEmpty artifact publicInput proof unit
          /\ validation.verifierQueryRowsRecoveredFromWitnessOpening
            artifact
            publicInput
            proof
            unit

theorem runtime_optional_constant_opening_checked_acceptance_contract
    {system : VerifierModel}
    (validation : RuntimeOptionalConstantOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeOptionalConstantOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOptionalConstantOpeningContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have openingAccepted :=
    validation.optionalConstantOpeningAcceptedImpliesOpeningSegmentBindingAccepted
      artifact
      publicInput
      proof
      accepted
  have openingBoundContract :=
    runtime_opening_segment_binding_checked_acceptance_opening_bound_contract
      validation.openingSegmentValidation
      artifact
      publicInput
      proof
      openingAccepted
  have constantSegments :=
    validation.optionalConstantOpeningAcceptedImpliesConstantOpeningSegmentsValid
      artifact
      publicInput
      proof
      accepted
  have absentContract :
      validation.constantOpeningSegmentAbsent artifact publicInput proof ->
        forall unit,
          validation.queriedUnit artifact publicInput proof unit ->
            validation.queriedUnitHasZeroConstantWidth artifact publicInput proof unit
              /\ ¬ validation.queriedUnitRequiresConstants artifact publicInput proof unit := by
    intro absent unit queried
    exact
      And.intro
        (validation.optionalConstantOpeningAcceptedAndAbsentImpliesQueriedUnitZeroWidth
          artifact
          publicInput
          proof
          unit
          accepted
          absent
          queried)
        (validation.optionalConstantOpeningAcceptedAndAbsentImpliesNoQueriedUnitRequiresConstants
          artifact
          publicInput
          proof
          unit
          accepted
          absent
          queried)
  have requiredUnitContract :
      forall unit,
        validation.queriedUnit artifact publicInput proof unit ->
          validation.queriedUnitRequiresConstants artifact publicInput proof unit ->
            validation.matchingConstantOpeningUnitPresent artifact publicInput proof unit := by
    intro unit queried requiresConstants
    exact
      validation.optionalConstantOpeningAcceptedAndRequiresConstantsImpliesMatchingUnitPresent
        artifact
        publicInput
        proof
        unit
        accepted
        queried
        requiresConstants
  have zeroWidthContract :
      forall unit,
        validation.queriedUnit artifact publicInput proof unit ->
          validation.queriedUnitHasZeroConstantWidth artifact publicInput proof unit ->
            validation.verifierConstantValuesEmpty artifact publicInput proof unit
              /\ validation.verifierQueryRowsRecoveredFromWitnessOpening
                artifact
                publicInput
                proof
                unit := by
    intro unit queried zeroWidth
    have rowsRecovered :=
      validation.optionalConstantOpeningAcceptedAndZeroWidthImpliesRowsRecoveredFromWitnessOpening
        artifact
        publicInput
        proof
        unit
        accepted
        queried
        zeroWidth
    exact
      And.intro
        (validation.optionalConstantOpeningAcceptedAndZeroWidthImpliesVerifierConstantValuesEmpty
          artifact
          publicInput
          proof
          unit
          accepted
          queried
          zeroWidth)
        rowsRecovered
  have presentContract :
      validation.constantOpeningSegmentPresent artifact publicInput proof ->
        validation.unexpectedConstantOpeningUnitsRejected artifact publicInput proof := by
    intro present
    exact
      validation.optionalConstantOpeningAcceptedAndPresentImpliesUnexpectedUnitsRejected
        artifact
        publicInput
        proof
        accepted
        present
  exact
    And.intro constantSegments
      (And.intro openingBoundContract.left
        (And.intro absentContract
          (And.intro requiredUnitContract
            (And.intro zeroWidthContract presentContract))))

theorem runtime_optional_constant_opening_checked_acceptance_absent_segment_contract
    {system : VerifierModel}
    (validation : RuntimeOptionalConstantOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeOptionalConstantOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.constantOpeningSegmentAbsent artifact publicInput proof ->
          RuntimeOptionalConstantOpeningAbsentSegmentContract
            system
            validation
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof accepted absent
  have optionalContract :=
    runtime_optional_constant_opening_checked_acceptance_contract
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro absent
      (And.intro optionalContract.right.left
        (optionalContract.right.right.left absent))

theorem runtime_optional_constant_opening_checked_acceptance_zero_width_query_contract
    {system : VerifierModel}
    (validation : RuntimeOptionalConstantOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeOptionalConstantOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOptionalConstantOpeningZeroWidthQueryContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have optionalContract :=
    runtime_optional_constant_opening_checked_acceptance_contract
      validation
      artifact
      publicInput
      proof
      accepted
  exact optionalContract.right.right.right.right.left

theorem runtime_optional_constant_opening_required_unit_has_matching_segment
    {system : VerifierModel}
    (validation : RuntimeOptionalConstantOpeningValidation system) :
    forall artifact publicInput proof unit,
      RuntimeOptionalConstantOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.queriedUnit artifact publicInput proof unit ->
          validation.queriedUnitRequiresConstants artifact publicInput proof unit ->
            validation.matchingConstantOpeningUnitPresent artifact publicInput proof unit := by
  intro artifact publicInput proof unit accepted queried requiresConstants
  have optionalContract :=
    runtime_optional_constant_opening_checked_acceptance_contract
      validation
      artifact
      publicInput
      proof
      accepted
  exact optionalContract.right.right.right.left unit queried requiresConstants

theorem runtime_optional_constant_opening_present_segment_rejects_unexpected_units
    {system : VerifierModel}
    (validation : RuntimeOptionalConstantOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeOptionalConstantOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.constantOpeningSegmentPresent artifact publicInput proof ->
          validation.unexpectedConstantOpeningUnitsRejected artifact publicInput proof := by
  intro artifact publicInput proof accepted present
  have optionalContract :=
    runtime_optional_constant_opening_checked_acceptance_contract
      validation
      artifact
      publicInput
      proof
      accepted
  exact optionalContract.right.right.right.right.right present

end Lzvm

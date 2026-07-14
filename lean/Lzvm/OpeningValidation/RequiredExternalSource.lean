/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.OpeningValidation

/-!
Combined opening contracts for proofs that require an external source.
-/

namespace Lzvm

theorem runtime_opening_required_external_source_evidence_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningValidation system) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof ->
        requiresExternalSource ->
          RuntimeOpeningEvidence
              system
              validation
              artifact
              publicInput
              proof
              requiresExternalSource
            /\ ExternalSourceOpeningEvidence
              system
              validation.runtimeSoundnessValidation.sourceValidation
              publicInput
              proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have requiredSound :=
    runtime_opening_required_external_source_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have core :=
    runtime_opening_required_external_source_verifier_core_contract
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
        (And.intro core requiredSound.right.right))

theorem runtime_opening_required_external_source_accepts_evidence_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningValidation system) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof ->
        requiresExternalSource ->
          system.accepts publicInput proof
            /\ RuntimeOpeningEvidence
              system
              validation
              artifact
              publicInput
              proof
              requiresExternalSource
            /\ ExternalSourceOpeningEvidence
              system
              validation.runtimeSoundnessValidation.sourceValidation
              publicInput
              proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have checkedContract :=
    runtime_opening_checked_acceptance_accepts_evidence_core_and_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have requiredContract :=
    runtime_opening_required_external_source_evidence_core_and_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  exact And.intro checkedContract.left requiredContract

end Lzvm

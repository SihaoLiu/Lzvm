/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.OpeningSegmentBinding

/-!
Concrete proof-segment identifiers for runtime opening segment binding.
-/

namespace Lzvm

theorem runtime_opening_segment_binding_checked_acceptance_concrete_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system)
    (binding :
      let transcriptValidation :=
        validation.openingValidation.runtimeSoundnessValidation.transcriptValidation
      RuntimeProofArtifactConcreteSegmentIdBinding
        transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof _requiresExternalSource accepted
  have openingAccepted :=
    runtime_opening_segment_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_checked_acceptance_concrete_segment_ids_allowed
      validation.openingValidation
      binding
      artifact
      publicInput
      proof
      _requiresExternalSource
      openingAccepted

theorem runtime_opening_segment_binding_checked_acceptance_concrete_core_sound_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningSegmentBindingValidation system)
    (binding :
      let transcriptValidation :=
        validation.openingValidation.runtimeSoundnessValidation.transcriptValidation
      RuntimeProofArtifactConcreteSegmentIdBinding
        transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        (RuntimeOpeningSegmentBindingEvidence
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
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof)
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    And.intro
      (runtime_opening_segment_binding_checked_acceptance_evidence_core_and_sound
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted)
      (runtime_opening_segment_binding_checked_acceptance_concrete_segment_ids_allowed
        validation
        binding
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted)

theorem runtime_opening_segment_binding_checked_acceptance_concrete_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningSegmentBindingValidation system)
    (binding :
      let transcriptValidation :=
        validation.openingValidation.runtimeSoundnessValidation.transcriptValidation
      RuntimeProofArtifactConcreteSegmentIdBinding
        transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeOpeningSegmentBindingEvidence
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
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have sound :=
    runtime_opening_segment_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have core :=
    runtime_opening_segment_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have concreteSegmentIds :=
    runtime_opening_segment_binding_checked_acceptance_concrete_segment_ids_allowed
      validation
      binding
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (And.intro
        (assumption_bundle_carries_required_semantic_evidence assumptions)
        (And.intro sound.left
          (And.intro sound.right.left
            (And.intro core
              (And.intro sound.right.right concreteSegmentIds)))))

end Lzvm

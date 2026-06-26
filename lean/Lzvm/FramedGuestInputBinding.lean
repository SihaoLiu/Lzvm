/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.EthBlockPublicInputBinding
import Lzvm.ProgramImageCacheBinding

/-!
Runtime framed guest-input binding obligations.
-/

namespace Lzvm

structure RuntimeFramedGuestInputBindingValidation (system : VerifierModel) where
  ethBlockValidation : RuntimeEthBlockPublicInputBindingValidation system
  programImageCacheValidation : RuntimeProgramImageCacheBindingValidation system
  framedGuestInputAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  framedGuestInputWellFormed : RuntimeArtifact -> PublicInput -> Proof -> Prop
  framedGuestInputProofSegmentPresent : RuntimeArtifact -> PublicInput -> Proof -> Prop
  framedGuestInputProofSegmentPayloadExact : RuntimeArtifact -> PublicInput -> Proof -> Prop
  framedGuestInputProofSegmentPayloadNonempty : RuntimeArtifact -> PublicInput -> Proof -> Prop
  framedGuestInputCoBoundWithEthBlock : RuntimeArtifact -> PublicInput -> Proof -> Prop
  framedGuestInputCoBoundWithProgramImage : RuntimeArtifact -> PublicInput -> Proof -> Prop
  framedGuestInputAcceptedImpliesEthBlockAccepted :
    forall artifact publicInput proof,
      framedGuestInputAccepted artifact publicInput proof ->
        ethBlockValidation.ethBlockBindingAccepted artifact publicInput proof
  framedGuestInputAcceptedImpliesProgramImageCacheAccepted :
    forall artifact publicInput proof,
      framedGuestInputAccepted artifact publicInput proof ->
        programImageCacheValidation.programImageCacheBindingAccepted artifact publicInput proof
  framedGuestInputAcceptedImpliesWellFormed :
    forall artifact publicInput proof,
      framedGuestInputAccepted artifact publicInput proof ->
        framedGuestInputWellFormed artifact publicInput proof
  framedGuestInputAcceptedImpliesProofSegmentPresent :
    forall artifact publicInput proof,
      framedGuestInputAccepted artifact publicInput proof ->
        framedGuestInputProofSegmentPresent artifact publicInput proof
  framedGuestInputAcceptedImpliesProofSegmentPayloadExact :
    forall artifact publicInput proof,
      framedGuestInputAccepted artifact publicInput proof ->
        framedGuestInputProofSegmentPayloadExact artifact publicInput proof
  framedGuestInputAcceptedImpliesProofSegmentPayloadNonempty :
    forall artifact publicInput proof,
      framedGuestInputAccepted artifact publicInput proof ->
        framedGuestInputProofSegmentPayloadNonempty artifact publicInput proof
  framedGuestInputAcceptedImpliesEthBlockCoBinding :
    forall artifact publicInput proof,
      framedGuestInputAccepted artifact publicInput proof ->
        framedGuestInputCoBoundWithEthBlock artifact publicInput proof
  framedGuestInputAcceptedImpliesProgramImageCoBinding :
    forall artifact publicInput proof,
      framedGuestInputAccepted artifact publicInput proof ->
        framedGuestInputCoBoundWithProgramImage artifact publicInput proof

def RuntimeFramedGuestInputBindingEvidence
    (system : VerifierModel)
    (validation : RuntimeFramedGuestInputBindingValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.framedGuestInputWellFormed artifact publicInput proof
    /\ validation.framedGuestInputProofSegmentPresent artifact publicInput proof
    /\ validation.framedGuestInputProofSegmentPayloadExact artifact publicInput proof
    /\ validation.framedGuestInputProofSegmentPayloadNonempty artifact publicInput proof
    /\ validation.framedGuestInputCoBoundWithEthBlock artifact publicInput proof
    /\ validation.framedGuestInputCoBoundWithProgramImage artifact publicInput proof
    /\ RuntimeEthBlockPublicInputBindingEvidence
      system
      validation.ethBlockValidation
      artifact
      publicInput
      proof
    /\ RuntimeProgramImageCacheBindingEvidence
      system
      validation.programImageCacheValidation
      artifact
      publicInput
      proof

def RuntimeFramedGuestInputBindingStructuralObligations
    (system : VerifierModel)
    (validation : RuntimeFramedGuestInputBindingValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeFramedGuestInputBindingEvidence
      system
      validation
      artifact
      publicInput
      proof
    /\ RuntimeEthBlockPublicInputBindingStructuralObligations
      system
      validation.ethBlockValidation
      artifact
      publicInput
      proof
    /\ RuntimeProgramImageCacheBindingStructuralObligations
      system
      validation.programImageCacheValidation
      artifact
      publicInput
      proof

def RuntimeFramedGuestInputBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeFramedGuestInputBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.framedGuestInputAccepted artifact publicInput proof

def RuntimeFramedGuestInputBindingSoundnessContract
    (system : VerifierModel)
    (validation : RuntimeFramedGuestInputBindingValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeFramedGuestInputBindingEvidence
      system
      validation
      artifact
      publicInput
      proof
    /\ RuntimeEthBlockPublicInputBindingSoundnessContract
      system
      validation.ethBlockValidation
      artifact
      publicInput
      proof
    /\ RuntimeProgramImageCacheBindingSoundnessContract
      system
      validation.programImageCacheValidation
      artifact
      publicInput
      proof
    /\ RuntimeVerifierCoreContract system publicInput proof
    /\ SoundWitness system publicInput proof

theorem runtime_framed_guest_input_binding_checked_acceptance_eth_block_acceptance
    {system : VerifierModel}
    (validation : RuntimeFramedGuestInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation.ethBlockValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.framedGuestInputAcceptedImpliesEthBlockAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_framed_guest_input_binding_checked_acceptance_program_image_cache_acceptance
    {system : VerifierModel}
    (validation : RuntimeFramedGuestInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProgramImageCacheBindingCheckedAcceptance
          system
          validation.programImageCacheValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.framedGuestInputAcceptedImpliesProgramImageCacheAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_framed_guest_input_binding_checked_acceptance_concrete_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeFramedGuestInputBindingValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.ethBlockValidation.proofArtifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  have ethAccepted :=
    runtime_framed_guest_input_binding_checked_acceptance_eth_block_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_eth_block_public_input_binding_checked_acceptance_concrete_segment_ids_allowed
      validation.ethBlockValidation
      binding
      artifact
      publicInput
      proof
      ethAccepted

theorem runtime_framed_guest_input_binding_checked_acceptance_evidence
    {system : VerifierModel}
    (validation : RuntimeFramedGuestInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFramedGuestInputBindingEvidence
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have ethAccepted :=
    runtime_framed_guest_input_binding_checked_acceptance_eth_block_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  have cacheAccepted :=
    runtime_framed_guest_input_binding_checked_acceptance_program_image_cache_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  have ethEvidence :=
    runtime_eth_block_public_input_binding_checked_acceptance_evidence
      validation.ethBlockValidation
      artifact
      publicInput
      proof
      ethAccepted
  have cacheEvidence :=
    runtime_program_image_cache_binding_checked_acceptance_evidence
      validation.programImageCacheValidation
      artifact
      publicInput
      proof
      cacheAccepted
  exact
    ⟨validation.framedGuestInputAcceptedImpliesWellFormed
        artifact
        publicInput
        proof
        accepted,
      validation.framedGuestInputAcceptedImpliesProofSegmentPresent
        artifact
        publicInput
        proof
        accepted,
      validation.framedGuestInputAcceptedImpliesProofSegmentPayloadExact
        artifact
        publicInput
        proof
        accepted,
      validation.framedGuestInputAcceptedImpliesProofSegmentPayloadNonempty
        artifact
        publicInput
        proof
        accepted,
      validation.framedGuestInputAcceptedImpliesEthBlockCoBinding
        artifact
        publicInput
        proof
        accepted,
      validation.framedGuestInputAcceptedImpliesProgramImageCoBinding
        artifact
        publicInput
        proof
        accepted,
      ethEvidence,
      cacheEvidence⟩

theorem runtime_framed_guest_input_binding_checked_acceptance_segment_present
    {system : VerifierModel}
    (validation : RuntimeFramedGuestInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.framedGuestInputProofSegmentPresent artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.framedGuestInputAcceptedImpliesProofSegmentPresent
      artifact
      publicInput
      proof
      accepted

theorem runtime_framed_guest_input_binding_checked_acceptance_segment_payload_exact
    {system : VerifierModel}
    (validation : RuntimeFramedGuestInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.framedGuestInputProofSegmentPayloadExact artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.framedGuestInputAcceptedImpliesProofSegmentPayloadExact
      artifact
      publicInput
      proof
      accepted

theorem runtime_framed_guest_input_binding_checked_acceptance_segment_payload_nonempty
    {system : VerifierModel}
    (validation : RuntimeFramedGuestInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.framedGuestInputProofSegmentPayloadNonempty artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.framedGuestInputAcceptedImpliesProofSegmentPayloadNonempty
      artifact
      publicInput
      proof
      accepted

theorem runtime_framed_guest_input_binding_checked_acceptance_eth_block_co_binding
    {system : VerifierModel}
    (validation : RuntimeFramedGuestInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.framedGuestInputCoBoundWithEthBlock artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.framedGuestInputAcceptedImpliesEthBlockCoBinding
      artifact
      publicInput
      proof
      accepted

theorem runtime_framed_guest_input_binding_checked_acceptance_program_image_cache_co_binding
    {system : VerifierModel}
    (validation : RuntimeFramedGuestInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.framedGuestInputCoBoundWithProgramImage artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.framedGuestInputAcceptedImpliesProgramImageCoBinding
      artifact
      publicInput
      proof
      accepted

theorem runtime_framed_guest_input_binding_checked_acceptance_structural_obligations
    {system : VerifierModel}
    (validation : RuntimeFramedGuestInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFramedGuestInputBindingStructuralObligations
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_framed_guest_input_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have ethAccepted :=
    runtime_framed_guest_input_binding_checked_acceptance_eth_block_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  have cacheAccepted :=
    runtime_framed_guest_input_binding_checked_acceptance_program_image_cache_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  have ethStructural :=
    runtime_eth_block_public_input_binding_checked_acceptance_structural_obligations
      validation.ethBlockValidation
      artifact
      publicInput
      proof
      ethAccepted
  have cacheStructural :=
    runtime_program_image_cache_binding_checked_acceptance_structural_obligations
      validation.programImageCacheValidation
      artifact
      publicInput
      proof
      cacheAccepted
  exact
    ⟨evidence, ethStructural, cacheStructural⟩

theorem runtime_framed_guest_input_binding_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeFramedGuestInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFramedGuestInputBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeEthBlockPublicInputBindingEvidence
            system
            validation.ethBlockValidation
            artifact
            publicInput
            proof
          /\ RuntimeProgramImageCacheBindingEvidence
            system
            validation.programImageCacheValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_framed_guest_input_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have ethAccepted :=
    runtime_framed_guest_input_binding_checked_acceptance_eth_block_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  have ethSound :=
    runtime_eth_block_public_input_binding_checked_acceptance_sound
      assumptions
      validation.ethBlockValidation
      artifact
      publicInput
      proof
      ethAccepted
  have core :=
    runtime_eth_block_public_input_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation.ethBlockValidation
      artifact
      publicInput
      proof
      ethAccepted
  have cacheEvidence : RuntimeProgramImageCacheBindingEvidence
      system
      validation.programImageCacheValidation
      artifact
      publicInput
      proof := by
    rcases evidence with ⟨_, _, _, _, _, _, _, cacheEvidence⟩
    exact cacheEvidence
  exact
    ⟨evidence,
      ethSound.left,
      cacheEvidence,
      core,
      ethSound.right.right.right⟩

theorem runtime_framed_guest_input_binding_checked_acceptance_soundness_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeFramedGuestInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFramedGuestInputBindingSoundnessContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_framed_guest_input_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have ethAccepted :=
    runtime_framed_guest_input_binding_checked_acceptance_eth_block_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  have cacheAccepted :=
    runtime_framed_guest_input_binding_checked_acceptance_program_image_cache_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  have ethContract :=
    runtime_eth_block_public_input_binding_checked_acceptance_soundness_contract
      assumptions
      validation.ethBlockValidation
      artifact
      publicInput
      proof
      ethAccepted
  have cacheContract :=
    runtime_program_image_cache_binding_checked_acceptance_soundness_contract
      assumptions
      validation.programImageCacheValidation
      artifact
      publicInput
      proof
      cacheAccepted
  have sound :=
    runtime_framed_guest_input_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    ⟨evidence,
      ethContract,
      cacheContract,
      sound.right.right.right.left,
      sound.right.right.right.right⟩

theorem runtime_framed_guest_input_binding_checked_acceptance_soundness_and_structural_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeFramedGuestInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFramedGuestInputBindingSoundnessContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeFramedGuestInputBindingStructuralObligations
            system
            validation
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (runtime_framed_guest_input_binding_checked_acceptance_soundness_contract
        assumptions
        validation
        artifact
        publicInput
        proof
        accepted)
      (runtime_framed_guest_input_binding_checked_acceptance_structural_obligations
        validation
        artifact
        publicInput
        proof
        accepted)

theorem runtime_framed_guest_input_binding_checked_acceptance_full_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeFramedGuestInputBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeFramedGuestInputBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFramedGuestInputBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeFramedGuestInputBindingStructuralObligations
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_framed_guest_input_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have structural :=
    runtime_framed_guest_input_binding_checked_acceptance_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    ⟨sound.left,
      structural,
      sound.right.right.right.left,
      sound.right.right.right.right⟩

end Lzvm

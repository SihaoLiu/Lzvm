/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.ProofArtifactBinding

/-!
Runtime program image cache binding obligations.
-/

namespace Lzvm

structure RuntimeProgramImageCacheBindingValidation (system : VerifierModel) where
  proofArtifactBindingValidation : RuntimeProofArtifactBindingValidation system
  programImageCacheBindingAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  programImageCacheSegmentMatches : RuntimeArtifact -> PublicInput -> Proof -> Prop
  programImageCachePublicValueMatches : RuntimeArtifact -> PublicInput -> Proof -> Prop
  programImageCacheSetupHashMatches : RuntimeArtifact -> PublicInput -> Proof -> Prop
  programImageCacheTreeRootCanonical : RuntimeArtifact -> PublicInput -> Proof -> Prop
  cacheBindingAcceptedImpliesProofArtifactBindingAccepted :
    forall artifact publicInput proof,
      programImageCacheBindingAccepted artifact publicInput proof ->
        proofArtifactBindingValidation.artifactBindingAccepted artifact publicInput proof
  cacheBindingAcceptedImpliesSegmentMatches :
    forall artifact publicInput proof,
      programImageCacheBindingAccepted artifact publicInput proof ->
        programImageCacheSegmentMatches artifact publicInput proof
  cacheBindingAcceptedImpliesPublicValueMatches :
    forall artifact publicInput proof,
      programImageCacheBindingAccepted artifact publicInput proof ->
        programImageCachePublicValueMatches artifact publicInput proof
  cacheBindingAcceptedImpliesSetupHashMatches :
    forall artifact publicInput proof,
      programImageCacheBindingAccepted artifact publicInput proof ->
        programImageCacheSetupHashMatches artifact publicInput proof
  cacheBindingAcceptedImpliesTreeRootCanonical :
    forall artifact publicInput proof,
      programImageCacheBindingAccepted artifact publicInput proof ->
        programImageCacheTreeRootCanonical artifact publicInput proof

def RuntimeProgramImageCacheBindingEvidence
    (_system : VerifierModel)
    (validation : RuntimeProgramImageCacheBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.programImageCacheSegmentMatches artifact publicInput proof
    /\ validation.programImageCachePublicValueMatches artifact publicInput proof
    /\ validation.programImageCacheSetupHashMatches artifact publicInput proof
    /\ validation.programImageCacheTreeRootCanonical artifact publicInput proof

def RuntimeProgramImageCacheBindingStructuralObligations
    (system : VerifierModel)
    (validation : RuntimeProgramImageCacheBindingValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeProgramImageCacheBindingEvidence
      system
      validation
      artifact
      publicInput
      proof
    /\ RuntimeProofArtifactBindingStructuralObligations
      system
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof

def RuntimeProgramImageCacheBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeProgramImageCacheBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.programImageCacheBindingAccepted artifact publicInput proof

def RuntimeProgramImageCacheBindingSoundnessContract
    (system : VerifierModel)
    (validation : RuntimeProgramImageCacheBindingValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeProgramImageCacheBindingEvidence
      system
      validation
      artifact
      publicInput
      proof
    /\ RuntimeProofArtifactBindingEvidence
      system
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
    /\ RuntimeArtifactEvidence
      system
      validation.proofArtifactBindingValidation.runtimeValidation
      artifact
      publicInput
      proof
    /\ system.accepts publicInput proof
    /\ validation.proofArtifactBindingValidation.proofContainerCanonical
      artifact
      publicInput
      proof
    /\ validation.proofArtifactBindingValidation.proofMetadataCanonical
      artifact
      publicInput
      proof
    /\ validation.proofArtifactBindingValidation.proofSegmentsPresent
      artifact
      publicInput
      proof
    /\ validation.proofArtifactBindingValidation.proofSegmentPayloadsNonempty
      artifact
      publicInput
      proof
    /\ validation.proofArtifactBindingValidation.proofSegmentIdsAllowed
      artifact
      publicInput
      proof
    /\ validation.proofArtifactBindingValidation.proofSegmentIdsUnique
      artifact
      publicInput
      proof
    /\ validation.proofArtifactBindingValidation.proofUnitValuesTraceIdentityCoverage
      artifact
      publicInput
      proof
    /\ RuntimeVerifierCoreContract system publicInput proof
    /\ SoundWitness system publicInput proof

theorem runtime_program_image_cache_binding_checked_acceptance_evidence
    {system : VerifierModel}
    (validation : RuntimeProgramImageCacheBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProgramImageCacheBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProgramImageCacheBindingEvidence
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    ⟨validation.cacheBindingAcceptedImpliesSegmentMatches
        artifact
        publicInput
        proof
        accepted,
      validation.cacheBindingAcceptedImpliesPublicValueMatches
        artifact
        publicInput
        proof
        accepted,
      validation.cacheBindingAcceptedImpliesSetupHashMatches
        artifact
        publicInput
        proof
        accepted,
      validation.cacheBindingAcceptedImpliesTreeRootCanonical
        artifact
        publicInput
        proof
        accepted⟩

theorem runtime_program_image_cache_binding_checked_acceptance_artifact_binding
    {system : VerifierModel}
    (validation : RuntimeProgramImageCacheBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProgramImageCacheBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation.proofArtifactBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.cacheBindingAcceptedImpliesProofArtifactBindingAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_program_image_cache_binding_checked_acceptance_artifact_finalized
    {system : VerifierModel}
    (validation : RuntimeProgramImageCacheBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProgramImageCacheBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactFinalized
          system
          validation.proofArtifactBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have artifactAccepted :=
    runtime_program_image_cache_binding_checked_acceptance_artifact_binding
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_proof_artifact_finalized_from_checked_acceptance
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted

theorem runtime_program_image_cache_binding_checked_acceptance_artifact_evidence_contract
    {system : VerifierModel}
    (validation : RuntimeProgramImageCacheBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProgramImageCacheBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProgramImageCacheBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingEvidence
            system
            validation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof accepted
  have cacheEvidence :=
    runtime_program_image_cache_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAccepted :=
    runtime_program_image_cache_binding_checked_acceptance_artifact_binding
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactEvidence :=
    runtime_proof_artifact_binding_checked_acceptance_evidence
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have runtimeEvidence :=
    runtime_proof_artifact_binding_evidence_implies_runtime_evidence
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactEvidence
  exact
    And.intro cacheEvidence
      (And.intro artifactEvidence runtimeEvidence)

theorem runtime_program_image_cache_binding_checked_acceptance_artifact_wellformed_contract
    {system : VerifierModel}
    (validation : RuntimeProgramImageCacheBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProgramImageCacheBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.proofArtifactBindingValidation.proofContainerCanonical artifact publicInput proof
          /\ validation.proofArtifactBindingValidation.proofMetadataCanonical
            artifact
            publicInput
            proof
          /\ validation.proofArtifactBindingValidation.proofSegmentsPresent
            artifact
            publicInput
            proof
          /\ validation.proofArtifactBindingValidation.proofSegmentPayloadsNonempty
            artifact
            publicInput
            proof
          /\ validation.proofArtifactBindingValidation.proofSegmentIdsAllowed
            artifact
            publicInput
            proof
          /\ validation.proofArtifactBindingValidation.proofSegmentIdsUnique
            artifact
            publicInput
            proof
          /\ validation.proofArtifactBindingValidation.proofUnitValuesTraceIdentityCoverage
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof accepted
  have artifactFinalized :=
    runtime_program_image_cache_binding_checked_acceptance_artifact_finalized
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_proof_artifact_finalized_structural_obligations
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactFinalized

theorem runtime_program_image_cache_binding_checked_acceptance_concrete_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeProgramImageCacheBindingValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.proofArtifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimeProgramImageCacheBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_proof_artifact_finalized_concrete_segment_ids_allowed
      validation.proofArtifactBindingValidation
      binding
      artifact
      publicInput
      proof
      (runtime_program_image_cache_binding_checked_acceptance_artifact_finalized
        validation
        artifact
        publicInput
        proof
        accepted)

theorem runtime_program_image_cache_binding_checked_acceptance_structural_obligations
    {system : VerifierModel}
    (validation : RuntimeProgramImageCacheBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProgramImageCacheBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProgramImageCacheBindingStructuralObligations
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have cacheEvidence :=
    runtime_program_image_cache_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactFinalized :=
    runtime_program_image_cache_binding_checked_acceptance_artifact_finalized
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactStructural :=
    runtime_proof_artifact_finalized_structural_obligations
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactFinalized
  exact
    And.intro cacheEvidence artifactStructural

theorem runtime_program_image_cache_binding_checked_acceptance_runtime_shape_contract
    {system : VerifierModel}
    (validation : RuntimeProgramImageCacheBindingValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.proofArtifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimeProgramImageCacheBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProgramImageCacheBindingStructuralObligations
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (runtime_program_image_cache_binding_checked_acceptance_structural_obligations
        validation
        artifact
        publicInput
        proof
        accepted)
      (runtime_program_image_cache_binding_checked_acceptance_concrete_segment_ids_allowed
        validation
        binding
        artifact
        publicInput
        proof
        accepted)

theorem runtime_program_image_cache_binding_checked_acceptance_unit_values_trace_identity_coverage
    {system : VerifierModel}
    (validation : RuntimeProgramImageCacheBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProgramImageCacheBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.proofArtifactBindingValidation.proofUnitValuesTraceIdentityCoverage
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have artifactAccepted :=
    runtime_program_image_cache_binding_checked_acceptance_artifact_binding
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_proof_artifact_binding_checked_acceptance_unit_values_trace_identity_coverage
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted

theorem runtime_program_image_cache_binding_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeProgramImageCacheBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProgramImageCacheBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProgramImageCacheBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingEvidence
            system
            validation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have evidenceContract :=
    runtime_program_image_cache_binding_checked_acceptance_artifact_evidence_contract
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactFinalized :=
    runtime_program_image_cache_binding_checked_acceptance_artifact_finalized
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactSound :=
    runtime_proof_artifact_finalized_full_contract
      assumptions
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactFinalized
  exact
    And.intro evidenceContract.left
      (And.intro artifactSound.left
        (And.intro artifactSound.right.right.left artifactSound.right.right.right))

theorem runtime_program_image_cache_binding_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeProgramImageCacheBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProgramImageCacheBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  have artifactFinalized :=
    runtime_program_image_cache_binding_checked_acceptance_artifact_finalized
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_proof_artifact_finalized_verifier_core_contract
      assumptions
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactFinalized

theorem runtime_program_image_cache_binding_checked_acceptance_soundness_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeProgramImageCacheBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProgramImageCacheBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProgramImageCacheBindingSoundnessContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_program_image_cache_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have wellformed :=
    runtime_program_image_cache_binding_checked_acceptance_artifact_wellformed_contract
      validation
      artifact
      publicInput
      proof
      accepted
  have core :=
    runtime_program_image_cache_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAccepted :=
    runtime_program_image_cache_binding_checked_acceptance_artifact_binding
      validation
      artifact
      publicInput
      proof
      accepted
  have runtimeAccepted :=
    runtime_proof_artifact_binding_checked_acceptance_runtime_accepted
      validation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have verifierAccepts :=
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      validation.proofArtifactBindingValidation.runtimeValidation
      artifact
      publicInput
      proof
      runtimeAccepted
  exact
    ⟨sound.left,
      sound.right.left,
      sound.right.right.left,
      verifierAccepts,
      wellformed.left,
      wellformed.right.left,
      wellformed.right.right.left,
      wellformed.right.right.right.left,
      wellformed.right.right.right.right.left,
      wellformed.right.right.right.right.right.left,
      wellformed.right.right.right.right.right.right,
      core,
      sound.right.right.right⟩

theorem runtime_program_image_cache_binding_checked_acceptance_full_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeProgramImageCacheBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProgramImageCacheBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProgramImageCacheBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingEvidence
            system
            validation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeProgramImageCacheBindingStructuralObligations
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_program_image_cache_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have structural :=
    runtime_program_image_cache_binding_checked_acceptance_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro sound.left
      (And.intro sound.right.left
        (And.intro structural sound.right.right))

theorem runtime_program_image_cache_binding_checked_acceptance_evidence_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeProgramImageCacheBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProgramImageCacheBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProgramImageCacheBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingEvidence
            system
            validation.proofArtifactBindingValidation
            artifact
            publicInput
            proof
          /\ RuntimeProgramImageCacheBindingStructuralObligations
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have fullContract :=
    runtime_program_image_cache_binding_checked_acceptance_full_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have coreContract :=
    runtime_program_image_cache_binding_checked_acceptance_verifier_core_contract
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
          (And.intro fullContract.right.right.right.left
            (And.intro coreContract fullContract.right.right.right.right))))

end Lzvm

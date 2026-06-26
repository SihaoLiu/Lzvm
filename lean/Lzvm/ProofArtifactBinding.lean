/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.Conformance
import Lzvm.ProofSegmentIds

/-!
Runtime proof artifact binding obligations.
-/

namespace Lzvm

structure RuntimeProofArtifactBindingValidation (system : VerifierModel) where
  runtimeValidation : RuntimeConformanceValidation system
  artifactBindingAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  setupHashMatches : RuntimeArtifact -> PublicInput -> Proof -> Prop
  publicValuesHashMatches : RuntimeArtifact -> PublicInput -> Proof -> Prop
  proofPayloadMatches : RuntimeArtifact -> PublicInput -> Proof -> Prop
  proofContainerCanonical : RuntimeArtifact -> PublicInput -> Proof -> Prop
  proofMetadataCanonical : RuntimeArtifact -> PublicInput -> Proof -> Prop
  proofSegmentsPresent : RuntimeArtifact -> PublicInput -> Proof -> Prop
  proofSegmentPayloadsNonempty : RuntimeArtifact -> PublicInput -> Proof -> Prop
  proofSegmentIdsAllowed : RuntimeArtifact -> PublicInput -> Proof -> Prop
  proofSegmentIdsUnique : RuntimeArtifact -> PublicInput -> Proof -> Prop
  proofUnitValuesTraceIdentityCoverage :
    RuntimeArtifact -> PublicInput -> Proof -> Prop
  bindingAcceptedImpliesRuntimeAccepted :
    forall artifact publicInput proof,
      artifactBindingAccepted artifact publicInput proof ->
        runtimeValidation.artifactAccepted artifact publicInput proof
  bindingAcceptedImpliesSetupHashMatches :
    forall artifact publicInput proof,
      artifactBindingAccepted artifact publicInput proof ->
        setupHashMatches artifact publicInput proof
  bindingAcceptedImpliesPublicValuesHashMatches :
    forall artifact publicInput proof,
      artifactBindingAccepted artifact publicInput proof ->
        publicValuesHashMatches artifact publicInput proof
  bindingAcceptedImpliesProofPayloadMatches :
    forall artifact publicInput proof,
      artifactBindingAccepted artifact publicInput proof ->
        proofPayloadMatches artifact publicInput proof
  bindingAcceptedImpliesProofContainerCanonical :
    forall artifact publicInput proof,
      artifactBindingAccepted artifact publicInput proof ->
        proofContainerCanonical artifact publicInput proof
  bindingAcceptedImpliesProofMetadataCanonical :
    forall artifact publicInput proof,
      artifactBindingAccepted artifact publicInput proof ->
        proofMetadataCanonical artifact publicInput proof
  bindingAcceptedImpliesProofSegmentsPresent :
    forall artifact publicInput proof,
      artifactBindingAccepted artifact publicInput proof ->
        proofSegmentsPresent artifact publicInput proof
  bindingAcceptedImpliesProofSegmentPayloadsNonempty :
    forall artifact publicInput proof,
      artifactBindingAccepted artifact publicInput proof ->
        proofSegmentPayloadsNonempty artifact publicInput proof
  bindingAcceptedImpliesProofSegmentIdsAllowed :
    forall artifact publicInput proof,
      artifactBindingAccepted artifact publicInput proof ->
        proofSegmentIdsAllowed artifact publicInput proof
  bindingAcceptedImpliesProofSegmentIdsUnique :
    forall artifact publicInput proof,
      artifactBindingAccepted artifact publicInput proof ->
        proofSegmentIdsUnique artifact publicInput proof
  bindingAcceptedImpliesProofUnitValuesTraceIdentityCoverage :
    forall artifact publicInput proof,
      artifactBindingAccepted artifact publicInput proof ->
        proofUnitValuesTraceIdentityCoverage artifact publicInput proof
  hashesMatchImpliesPublicInputMatches :
    forall artifact publicInput proof,
      setupHashMatches artifact publicInput proof ->
        publicValuesHashMatches artifact publicInput proof ->
          runtimeValidation.artifactPublicInputMatches artifact publicInput proof
  proofPayloadImpliesProofMatches :
    forall artifact publicInput proof,
      proofPayloadMatches artifact publicInput proof ->
        runtimeValidation.artifactProofMatches artifact publicInput proof

def RuntimeProofArtifactBindingValidationAgreement
    {system : VerifierModel}
    (left right : RuntimeProofArtifactBindingValidation system) : Prop :=
  RuntimeConformanceValidationAgreement
      left.runtimeValidation
      right.runtimeValidation
    /\ (forall artifact publicInput proof,
      left.artifactBindingAccepted artifact publicInput proof <->
        right.artifactBindingAccepted artifact publicInput proof)
    /\ (forall artifact publicInput proof,
      left.setupHashMatches artifact publicInput proof <->
        right.setupHashMatches artifact publicInput proof)
    /\ (forall artifact publicInput proof,
      left.publicValuesHashMatches artifact publicInput proof <->
        right.publicValuesHashMatches artifact publicInput proof)
    /\ (forall artifact publicInput proof,
      left.proofPayloadMatches artifact publicInput proof <->
        right.proofPayloadMatches artifact publicInput proof)
    /\ (forall artifact publicInput proof,
      left.proofContainerCanonical artifact publicInput proof <->
        right.proofContainerCanonical artifact publicInput proof)
    /\ (forall artifact publicInput proof,
      left.proofMetadataCanonical artifact publicInput proof <->
        right.proofMetadataCanonical artifact publicInput proof)
    /\ (forall artifact publicInput proof,
      left.proofSegmentsPresent artifact publicInput proof <->
        right.proofSegmentsPresent artifact publicInput proof)
    /\ (forall artifact publicInput proof,
      left.proofSegmentPayloadsNonempty artifact publicInput proof <->
        right.proofSegmentPayloadsNonempty artifact publicInput proof)
    /\ (forall artifact publicInput proof,
      left.proofSegmentIdsAllowed artifact publicInput proof <->
        right.proofSegmentIdsAllowed artifact publicInput proof)
    /\ (forall artifact publicInput proof,
      left.proofSegmentIdsUnique artifact publicInput proof <->
        right.proofSegmentIdsUnique artifact publicInput proof)
    /\ (forall artifact publicInput proof,
      left.proofUnitValuesTraceIdentityCoverage artifact publicInput proof <->
        right.proofUnitValuesTraceIdentityCoverage artifact publicInput proof)

def RuntimeProofArtifactBindingEvidence
    (_system : VerifierModel)
    (validation : RuntimeProofArtifactBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.setupHashMatches artifact publicInput proof
    /\ validation.publicValuesHashMatches artifact publicInput proof
    /\ validation.proofPayloadMatches artifact publicInput proof

def RuntimeProofArtifactBindingStructuralObligations
    (_system : VerifierModel)
    (validation : RuntimeProofArtifactBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.proofContainerCanonical artifact publicInput proof
    /\ validation.proofMetadataCanonical artifact publicInput proof
    /\ validation.proofSegmentsPresent artifact publicInput proof
    /\ validation.proofSegmentPayloadsNonempty artifact publicInput proof
    /\ validation.proofSegmentIdsAllowed artifact publicInput proof
    /\ validation.proofSegmentIdsUnique artifact publicInput proof
    /\ validation.proofUnitValuesTraceIdentityCoverage artifact publicInput proof

def RuntimeProofArtifactBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeProofArtifactBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.artifactBindingAccepted artifact publicInput proof

def RuntimeProofArtifactFinalized
    (system : VerifierModel)
    (validation : RuntimeProofArtifactBindingValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeProofArtifactBindingCheckedAcceptance
      system
      validation
      artifact
      publicInput
      proof
    /\ RuntimeProofArtifactBindingStructuralObligations
      system
      validation
      artifact
      publicInput
      proof

def RuntimeProofArtifactConcreteSegmentIdsAllowed
    (proof : Proof) : Prop :=
  forall id,
    id ∈ proof.segmentIds ->
      IsAllowedProofSegmentId id

structure RuntimeProofArtifactConcreteSegmentIdBinding
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) where
  proofSegmentIdsAllowedImpliesConcrete :
    forall artifact publicInput proof,
      validation.proofSegmentIdsAllowed artifact publicInput proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed
          proof

theorem runtime_proof_artifact_binding_checked_acceptance_concrete_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system)
    (binding : RuntimeProofArtifactConcreteSegmentIdBinding validation) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed
          proof := by
  intro artifact publicInput proof accepted
  exact
    binding.proofSegmentIdsAllowedImpliesConcrete
      artifact
      publicInput
      proof
      (validation.bindingAcceptedImpliesProofSegmentIdsAllowed
        artifact
        publicInput
        proof
        accepted)

theorem runtime_proof_artifact_finalized_concrete_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system)
    (binding : RuntimeProofArtifactConcreteSegmentIdBinding validation) :
    forall artifact publicInput proof,
      RuntimeProofArtifactFinalized
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed
          proof := by
  intro artifact publicInput proof finalized
  exact
    runtime_proof_artifact_binding_checked_acceptance_concrete_segment_ids_allowed
      validation
      binding
      artifact
      publicInput
      proof
      finalized.left

theorem runtime_proof_artifact_binding_checked_acceptance_evidence
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactBindingEvidence
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (validation.bindingAcceptedImpliesSetupHashMatches
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (validation.bindingAcceptedImpliesPublicValuesHashMatches
          artifact
          publicInput
          proof
          accepted)
        (validation.bindingAcceptedImpliesProofPayloadMatches
          artifact
          publicInput
          proof
          accepted))

theorem runtime_proof_artifact_binding_evidence_implies_runtime_evidence
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeArtifactEvidence
          system
          validation.runtimeValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof evidence
  exact
    And.intro
      (validation.hashesMatchImpliesPublicInputMatches
        artifact
        publicInput
        proof
        evidence.left
        evidence.right.left)
      (validation.proofPayloadImpliesProofMatches
        artifact
        publicInput
        proof
        evidence.right.right)

theorem runtime_proof_artifact_binding_checked_acceptance_runtime_evidence
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeArtifactEvidence
          system
          validation.runtimeValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_proof_artifact_binding_evidence_implies_runtime_evidence
      validation
      artifact
      publicInput
      proof
      (runtime_proof_artifact_binding_checked_acceptance_evidence
        validation
        artifact
        publicInput
        proof
        accepted)

theorem runtime_proof_artifact_binding_checked_acceptance_runtime_accepted
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.runtimeValidation.artifactAccepted artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.bindingAcceptedImpliesRuntimeAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_proof_artifact_binding_checked_acceptance_container_canonical
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.proofContainerCanonical artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.bindingAcceptedImpliesProofContainerCanonical
      artifact
      publicInput
      proof
      accepted

theorem runtime_proof_artifact_binding_checked_acceptance_metadata_canonical
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.proofMetadataCanonical artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.bindingAcceptedImpliesProofMetadataCanonical
      artifact
      publicInput
      proof
      accepted

theorem runtime_proof_artifact_binding_checked_acceptance_segments_present
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.proofSegmentsPresent artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.bindingAcceptedImpliesProofSegmentsPresent
      artifact
      publicInput
      proof
      accepted

theorem runtime_proof_artifact_binding_checked_acceptance_segment_payloads_nonempty
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.proofSegmentPayloadsNonempty artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.bindingAcceptedImpliesProofSegmentPayloadsNonempty
      artifact
      publicInput
      proof
      accepted

theorem runtime_proof_artifact_binding_checked_acceptance_segment_ids_unique
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.proofSegmentIdsUnique artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.bindingAcceptedImpliesProofSegmentIdsUnique
      artifact
      publicInput
      proof
      accepted

theorem runtime_proof_artifact_binding_checked_acceptance_unit_values_trace_identity_coverage
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.proofUnitValuesTraceIdentityCoverage
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.bindingAcceptedImpliesProofUnitValuesTraceIdentityCoverage
      artifact
      publicInput
      proof
      accepted

theorem runtime_proof_artifact_binding_checked_acceptance_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.proofSegmentIdsAllowed artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.bindingAcceptedImpliesProofSegmentIdsAllowed
      artifact
      publicInput
      proof
      accepted

theorem runtime_proof_artifact_binding_checked_acceptance_structural_obligations
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactBindingStructuralObligations
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (validation.bindingAcceptedImpliesProofContainerCanonical
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (validation.bindingAcceptedImpliesProofMetadataCanonical
          artifact
          publicInput
          proof
          accepted)
        (And.intro
          (validation.bindingAcceptedImpliesProofSegmentsPresent
            artifact
            publicInput
            proof
            accepted)
          (And.intro
            (validation.bindingAcceptedImpliesProofSegmentPayloadsNonempty
              artifact
              publicInput
              proof
              accepted)
            (And.intro
              (validation.bindingAcceptedImpliesProofSegmentIdsAllowed
                artifact
                publicInput
                proof
                accepted)
              (And.intro
                (validation.bindingAcceptedImpliesProofSegmentIdsUnique
                  artifact
                  publicInput
                  proof
                  accepted)
                (validation.bindingAcceptedImpliesProofUnitValuesTraceIdentityCoverage
                  artifact
                  publicInput
                  proof
                  accepted))))))

theorem runtime_proof_artifact_finalized_from_checked_acceptance
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactFinalized
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro accepted
      (runtime_proof_artifact_binding_checked_acceptance_structural_obligations
        validation
        artifact
        publicInput
        proof
        accepted)

theorem runtime_proof_artifact_finalized_structural_obligations
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactFinalized
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactBindingStructuralObligations
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof finalized
  exact finalized.right

theorem runtime_proof_artifact_finalized_checked_acceptance
    {system : VerifierModel}
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactFinalized
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof finalized
  exact finalized.left

theorem runtime_proof_artifact_binding_checked_acceptance_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactSoundnessObligations
            system
            validation.runtimeValidation
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof accepted
  have bindingEvidence :=
    runtime_proof_artifact_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have runtimeAccepted :=
    validation.bindingAcceptedImpliesRuntimeAccepted
      artifact
      publicInput
      proof
      accepted
  have runtimeObligations :=
    runtime_artifact_checked_acceptance_obligations
      assumptions
      validation.runtimeValidation
      artifact
      publicInput
      proof
      runtimeAccepted
  exact
    And.intro bindingEvidence runtimeObligations

theorem runtime_proof_artifact_binding_checked_acceptance_soundness_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeArtifactSoundnessObligations
          system
          validation.runtimeValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    (runtime_proof_artifact_binding_checked_acceptance_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted).right

theorem runtime_proof_artifact_binding_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.runtimeValidation
            artifact
            publicInput
            proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have obligations :=
    runtime_proof_artifact_binding_checked_acceptance_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have verifierAccepts := obligations.right.right.left
  exact
    And.intro obligations.left
      (And.intro obligations.right.left
        (abstract_verifier_sound assumptions publicInput proof verifierAccepts))

theorem runtime_proof_artifact_binding_checked_acceptance_full_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingStructuralObligations
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.runtimeValidation
            artifact
            publicInput
            proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_proof_artifact_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have structural :=
    runtime_proof_artifact_binding_checked_acceptance_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro sound.left
      (And.intro structural sound.right)

theorem runtime_proof_artifact_binding_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  have obligations :=
    runtime_proof_artifact_binding_checked_acceptance_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact obligations.right.right.right

theorem runtime_proof_artifact_finalized_full_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactFinalized
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeProofArtifactBindingStructuralObligations
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeArtifactEvidence
            system
            validation.runtimeValidation
            artifact
            publicInput
            proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof finalized
  have sound :=
    runtime_proof_artifact_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      finalized.left
  exact
    And.intro sound.left
      (And.intro finalized.right sound.right)

theorem runtime_proof_artifact_finalized_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeProofArtifactBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeProofArtifactFinalized
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof finalized
  exact
    runtime_proof_artifact_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      finalized.left

end Lzvm

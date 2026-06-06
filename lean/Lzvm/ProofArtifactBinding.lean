/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.Conformance

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
  hashesMatchImpliesPublicInputMatches :
    forall artifact publicInput proof,
      setupHashMatches artifact publicInput proof ->
        publicValuesHashMatches artifact publicInput proof ->
          runtimeValidation.artifactPublicInputMatches artifact publicInput proof
  proofPayloadImpliesProofMatches :
    forall artifact publicInput proof,
      proofPayloadMatches artifact publicInput proof ->
        runtimeValidation.artifactProofMatches artifact publicInput proof

def RuntimeProofArtifactBindingEvidence
    (_system : VerifierModel)
    (validation : RuntimeProofArtifactBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.setupHashMatches artifact publicInput proof
    /\ validation.publicValuesHashMatches artifact publicInput proof
    /\ validation.proofPayloadMatches artifact publicInput proof

def RuntimeProofArtifactBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeProofArtifactBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.artifactBindingAccepted artifact publicInput proof

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

end Lzvm

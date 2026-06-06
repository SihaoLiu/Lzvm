/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.Conformance
import Lzvm.ExternalSource

/-!
Runtime artifact conformance combined with external source opening evidence.
-/

namespace Lzvm

def RuntimeExternalSourceCheckedAcceptance
    (system : VerifierModel)
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeArtifactCheckedAcceptance
      system
      runtimeValidation
      artifact
      publicInput
      proof
    /\ ExternalSourceOpeningEvidence system sourceValidation publicInput proof

def RuntimeGuardedExternalSourceCheckedAcceptance
    (system : VerifierModel)
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof)
    (requiresExternalSource : Prop) : Prop :=
  RuntimeArtifactCheckedAcceptance
      system
      runtimeValidation
      artifact
      publicInput
      proof
    /\ ExternalSourceOpeningRequirement
      system
      sourceValidation
      publicInput
      proof
      requiresExternalSource

theorem runtime_external_source_checked_acceptance_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeExternalSourceCheckedAcceptance
          system
          runtimeValidation
          sourceValidation
          artifact
          publicInput
          proof ->
        RuntimeArtifactSoundnessObligations
            system
            runtimeValidation
            artifact
            publicInput
            proof
          /\ ExternalSourceOpeningSoundnessObligations
            system
            sourceValidation
            publicInput
            proof := by
  intro artifact publicInput proof checked
  have artifactAccepted := checked.left
  have externalEvidence := checked.right
  have artifactObligations :=
    runtime_artifact_checked_acceptance_obligations
      assumptions
      runtimeValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have verifierAccepts :=
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      runtimeValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have externalChecked :
      ExternalSourceOpeningCheckedAcceptance
        system
        sourceValidation
        publicInput
        proof :=
    And.intro verifierAccepts externalEvidence
  have externalObligations :=
    external_source_opening_checked_acceptance_obligations
      assumptions
      sourceValidation
      publicInput
      proof
      externalChecked
  exact And.intro artifactObligations externalObligations

theorem runtime_external_source_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeExternalSourceCheckedAcceptance
          system
          runtimeValidation
          sourceValidation
          artifact
          publicInput
          proof ->
        RuntimeArtifactEvidence
            system
            runtimeValidation
            artifact
            publicInput
            proof
          /\ ExternalSourceOpeningEvidence system sourceValidation publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof checked
  have artifactAccepted := checked.left
  have externalEvidence := checked.right
  have artifactEvidence :=
    runtime_artifact_checked_acceptance_evidence
      runtimeValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have verifierAccepts :=
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      runtimeValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have externalChecked :
      ExternalSourceOpeningCheckedAcceptance
        system
        sourceValidation
        publicInput
        proof :=
    And.intro verifierAccepts externalEvidence
  have externalSound :=
    external_source_opening_checked_acceptance_sound
      assumptions
      sourceValidation
      publicInput
      proof
      externalChecked
  exact
    And.intro artifactEvidence
      (And.intro externalEvidence
        (And.intro externalSound.right.left externalSound.right.right))

theorem runtime_guarded_external_source_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeGuardedExternalSourceCheckedAcceptance
          system
          runtimeValidation
          sourceValidation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeArtifactEvidence
            system
            runtimeValidation
            artifact
            publicInput
            proof
          /\ ExternalSourceOpeningRequirement
            system
            sourceValidation
            publicInput
            proof
            requiresExternalSource
          /\ system.pcsOpeningsValid publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have artifactAccepted := checked.left
  have sourceRequirement := checked.right
  have artifactEvidence :=
    runtime_artifact_checked_acceptance_evidence
      runtimeValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have verifierAccepts :=
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      runtimeValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have pcsOpenings :=
    assumptions.crypto.pcs_opening_sound publicInput proof verifierAccepts
  have soundWitness :=
    abstract_verifier_sound assumptions publicInput proof verifierAccepts
  exact
    And.intro artifactEvidence
      (And.intro sourceRequirement (And.intro pcsOpenings soundWitness))

end Lzvm

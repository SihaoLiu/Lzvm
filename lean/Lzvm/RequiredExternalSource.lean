/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.RuntimeExternalSource

/-!
Required external source evidence for guarded runtime acceptance.
-/

namespace Lzvm

theorem runtime_guarded_external_source_required_evidence
    {system : VerifierModel}
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
        requiresExternalSource ->
          ExternalSourceOpeningEvidence system sourceValidation publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  exact
    external_source_opening_requirement_implies_evidence
      sourceValidation
      publicInput
      proof
      requiresExternalSource
      checked.right
      required

theorem runtime_guarded_external_source_required_sound
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
        requiresExternalSource ->
          RuntimeArtifactEvidence
              system
              runtimeValidation
              artifact
              publicInput
              proof
            /\ ExternalSourceOpeningEvidence system sourceValidation publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have artifactAccepted := checked.left
  have artifactEvidence :=
    runtime_artifact_checked_acceptance_evidence
      runtimeValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have externalEvidence :=
    runtime_guarded_external_source_required_evidence
      runtimeValidation
      sourceValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  have pcsOpenings :=
    external_source_opening_evidence_implies_pcs_openings
      sourceValidation
      publicInput
      proof
      externalEvidence
  have verifierAccepts :=
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      runtimeValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have soundWitness :=
    abstract_verifier_sound assumptions publicInput proof verifierAccepts
  exact
    And.intro artifactEvidence
      (And.intro externalEvidence (And.intro pcsOpenings soundWitness))

theorem runtime_guarded_external_source_required_verifier_core_contract
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
        requiresExternalSource ->
          RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have sound :=
    runtime_guarded_external_source_required_sound
      assumptions
      runtimeValidation
      sourceValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact sound_witness_implies_verifier_core_contract sound.right.right.right

end Lzvm

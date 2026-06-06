/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.Soundness

/-!
Runtime conformance bridge for checked proof artifacts.
-/

namespace Lzvm

/-!
The checker-facing artifact model is intentionally small. Concrete Rust and
CUDA checks should later discharge the validation predicates below, while the
Lean theorem records how accepted runtime artifacts enter the abstract verifier
soundness model.
-/

structure RuntimeArtifact where
  id : Nat
deriving DecidableEq, Repr

structure RuntimeConformanceValidation (system : VerifierModel) where
  artifactAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  artifactPublicInputMatches : RuntimeArtifact -> PublicInput -> Proof -> Prop
  artifactProofMatches : RuntimeArtifact -> PublicInput -> Proof -> Prop
  artifactAcceptedImpliesVerifierAccepts :
    forall artifact publicInput proof,
      artifactAccepted artifact publicInput proof ->
        system.accepts publicInput proof
  artifactAcceptedImpliesPublicInputMatches :
    forall artifact publicInput proof,
      artifactAccepted artifact publicInput proof ->
        artifactPublicInputMatches artifact publicInput proof
  artifactAcceptedImpliesProofMatches :
    forall artifact publicInput proof,
      artifactAccepted artifact publicInput proof ->
        artifactProofMatches artifact publicInput proof

def RuntimeArtifactEvidence
    (system : VerifierModel)
    (validation : RuntimeConformanceValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.artifactPublicInputMatches artifact publicInput proof
    /\ validation.artifactProofMatches artifact publicInput proof

def RuntimeArtifactCheckedAcceptance
    (system : VerifierModel)
    (validation : RuntimeConformanceValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.artifactAccepted artifact publicInput proof

def RuntimeArtifactSoundnessObligations
    (system : VerifierModel)
    (validation : RuntimeConformanceValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeArtifactEvidence system validation artifact publicInput proof
    /\ system.accepts publicInput proof
    /\ system.transcriptBound publicInput proof
    /\ system.publicInputBound publicInput proof
    /\ system.pcsOpeningsValid publicInput proof
    /\ system.friQueriesValid publicInput proof

theorem runtime_artifact_checked_acceptance_implies_verifier_accepts
    {system : VerifierModel}
    (validation : RuntimeConformanceValidation system) :
    forall artifact publicInput proof,
      RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof ->
        system.accepts publicInput proof := by
  intro artifact publicInput proof artifactAccepted
  exact
    validation.artifactAcceptedImpliesVerifierAccepts
      artifact
      publicInput
      proof
      artifactAccepted

theorem runtime_artifact_checked_acceptance_evidence
    {system : VerifierModel}
    (validation : RuntimeConformanceValidation system) :
    forall artifact publicInput proof,
      RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof ->
        RuntimeArtifactEvidence system validation artifact publicInput proof := by
  intro artifact publicInput proof artifactAccepted
  exact
    And.intro
      (validation.artifactAcceptedImpliesPublicInputMatches
        artifact
        publicInput
        proof
        artifactAccepted)
      (validation.artifactAcceptedImpliesProofMatches
        artifact
        publicInput
        proof
        artifactAccepted)

theorem runtime_artifact_checked_acceptance_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeConformanceValidation system) :
    forall artifact publicInput proof,
      RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof ->
        RuntimeArtifactSoundnessObligations
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof artifactAccepted
  have verifierAccepts :=
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      validation
      artifact
      publicInput
      proof
      artifactAccepted
  have evidence :=
    runtime_artifact_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      artifactAccepted
  have transcriptBound :=
    assumptions.crypto.transcript_binding publicInput proof verifierAccepts
  have publicInputBound :=
    assumptions.semantic.public_input_binding publicInput proof verifierAccepts
  have pcsOpenings :=
    assumptions.crypto.pcs_opening_sound publicInput proof verifierAccepts
  have friQueries :=
    assumptions.crypto.fri_query_sound publicInput proof verifierAccepts
  exact
    And.intro evidence
      (And.intro verifierAccepts
        (And.intro transcriptBound
          (And.intro publicInputBound
            (And.intro pcsOpenings friQueries))))

theorem runtime_artifact_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeConformanceValidation system) :
    forall artifact publicInput proof,
      RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof ->
        RuntimeArtifactSoundnessObligations
          system
          validation
          artifact
          publicInput
          proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof artifactAccepted
  have obligations :=
    runtime_artifact_checked_acceptance_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      artifactAccepted
  have verifierAccepts := obligations.right.left
  exact
    And.intro obligations
      (abstract_verifier_sound assumptions publicInput proof verifierAccepts)

end Lzvm

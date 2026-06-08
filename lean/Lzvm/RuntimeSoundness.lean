/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AssumptionAudit
import Lzvm.RequiredExternalSource
import Lzvm.TranscriptBinding

/-!
Integrated runtime soundness theorem for checked proof artifacts.
-/

namespace Lzvm

structure RuntimeSoundnessValidation (system : VerifierModel) where
  transcriptValidation : RuntimeTranscriptBindingValidation system
  sourceValidation : ExternalSourceOpeningValidation system

def RuntimeSoundnessCheckedAcceptance
    (system : VerifierModel)
    (validation : RuntimeSoundnessValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof)
    (requiresExternalSource : Prop) : Prop :=
  RuntimeTranscriptBindingCheckedAcceptance
      system
      validation.transcriptValidation
      artifact
      publicInput
      proof
    /\ ExternalSourceOpeningRequirement
      system
      validation.sourceValidation
      publicInput
      proof
      requiresExternalSource

def RuntimeSoundnessEvidence
    (system : VerifierModel)
    (validation : RuntimeSoundnessValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof)
    (requiresExternalSource : Prop) : Prop :=
  RuntimeTranscriptBindingEvidence
      system
      validation.transcriptValidation
      artifact
      publicInput
      proof
    /\ RuntimeArtifactEvidence
      system
      validation.transcriptValidation.artifactBindingValidation.runtimeValidation
      artifact
      publicInput
      proof
    /\ system.transcriptBound publicInput proof
    /\ ExternalSourceOpeningRequirement
      system
      validation.sourceValidation
      publicInput
      proof
      requiresExternalSource
    /\ system.pcsOpeningsValid publicInput proof
    /\ system.friQueriesValid publicInput proof

theorem runtime_soundness_evidence_implies_pcs_and_fri
    {system : VerifierModel}
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact evidence.right.right.right.right

theorem runtime_soundness_checked_acceptance_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeSoundnessEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource checked
  have transcriptAccepted := checked.left
  have sourceRequirement := checked.right
  have transcriptSound :=
    runtime_transcript_binding_checked_acceptance_sound
      assumptions
      validation.transcriptValidation
      artifact
      publicInput
      proof
      transcriptAccepted
  have artifactAccepted :=
    validation.transcriptValidation.transcriptAcceptedImpliesArtifactBindingAccepted
      artifact
      publicInput
      proof
      transcriptAccepted
  have runtimeAccepted :=
    validation.transcriptValidation.artifactBindingValidation.bindingAcceptedImpliesRuntimeAccepted
      artifact
      publicInput
      proof
      artifactAccepted
  have verifierAccepts :=
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      validation.transcriptValidation.artifactBindingValidation.runtimeValidation
      artifact
      publicInput
      proof
      runtimeAccepted
  have pcsOpenings :=
    assumptions.crypto.pcs_opening_sound publicInput proof verifierAccepts
  have friQueries :=
    assumptions.crypto.fri_query_sound publicInput proof verifierAccepts
  exact
    And.intro transcriptSound.left
      (And.intro transcriptSound.right.left
        (And.intro transcriptSound.right.right.left
          (And.intro sourceRequirement
            (And.intro pcsOpenings friQueries))))

theorem runtime_soundness_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeSoundnessEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have evidence :=
    runtime_soundness_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have transcriptSound :=
    runtime_transcript_binding_checked_acceptance_sound
      assumptions
      validation.transcriptValidation
      artifact
      publicInput
      proof
      checked.left
  exact And.intro evidence transcriptSound.right.right.right

theorem runtime_soundness_checked_acceptance_audited_assumptions
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RuntimeSoundnessEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have audited :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have sound :=
    runtime_soundness_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  exact And.intro audited sound

theorem runtime_soundness_checked_acceptance_core_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have sound :=
    runtime_soundness_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  rcases sound.right with
    ⟨_witness,
      _trace,
      _constraints,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid,
      _traceConsistent,
      _constraintsSatisfied,
      _witnessMatchesTrace⟩
  exact
    ⟨transcriptBound, publicInputBound, pcsOpeningsValid, friQueriesValid⟩

theorem runtime_soundness_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  exact
    runtime_soundness_checked_acceptance_core_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked

theorem runtime_soundness_required_external_source_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          RuntimeSoundnessEvidence
              system
              validation
              artifact
              publicInput
              proof
              requiresExternalSource
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have evidence :=
    runtime_soundness_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have externalEvidence :=
    external_source_opening_requirement_implies_evidence
      validation.sourceValidation
      publicInput
      proof
      requiresExternalSource
      checked.right
      required
  have sound :=
    runtime_soundness_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  exact And.intro evidence (And.intro externalEvidence sound.right)

theorem runtime_soundness_required_external_source_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have sound :=
    runtime_soundness_required_external_source_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact sound_witness_implies_verifier_core_contract sound.right.right

end Lzvm

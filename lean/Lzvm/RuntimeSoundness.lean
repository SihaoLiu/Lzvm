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
    /\ system.publicInputBound publicInput proof
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
  exact evidence.right.right.right.right.right

theorem runtime_soundness_evidence_implies_runtime_artifact_evidence
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
        RuntimeArtifactEvidence
          system
          validation.transcriptValidation.artifactBindingValidation.runtimeValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact evidence.right.left

theorem runtime_soundness_evidence_implies_transcript_bound
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
        system.transcriptBound publicInput proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact evidence.right.right.left

theorem runtime_soundness_evidence_implies_public_input_bound
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
        system.publicInputBound publicInput proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact evidence.right.right.right.left

theorem runtime_soundness_evidence_implies_external_source_requirement
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
        ExternalSourceOpeningRequirement
          system
          validation.sourceValidation
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact evidence.right.right.right.right.left

theorem runtime_soundness_evidence_implies_core_obligations
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
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact
    And.intro evidence.right.right.left
      (And.intro evidence.right.right.right.left
        (And.intro
          evidence.right.right.right.right.right.left
          evidence.right.right.right.right.right.right))

theorem runtime_soundness_evidence_implies_runtime_artifact_core_contract
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
        RuntimeArtifactEvidence
          system
          validation.transcriptValidation.artifactBindingValidation.runtimeValidation
          artifact
          publicInput
          proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  exact
    And.intro
      (runtime_soundness_evidence_implies_runtime_artifact_evidence
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        evidence)
      (runtime_soundness_evidence_implies_core_obligations
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        evidence)

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
  have publicInputBound :=
    (sound_witness_implies_verifier_core_contract
      transcriptSound.right.right.right).right.left
  exact
    And.intro transcriptSound.left
      (And.intro transcriptSound.right.left
        (And.intro transcriptSound.right.right.left
          (And.intro publicInputBound
            (And.intro sourceRequirement
              (And.intro pcsOpenings friQueries)))))

theorem runtime_soundness_checked_acceptance_runtime_artifact_evidence
    {system : VerifierModel}
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeArtifactEvidence
          system
          validation.transcriptValidation.artifactBindingValidation.runtimeValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have artifactAccepted :=
    validation.transcriptValidation.transcriptAcceptedImpliesArtifactBindingAccepted
      artifact
      publicInput
      proof
      checked.left
  have runtimeAccepted :=
    validation.transcriptValidation.artifactBindingValidation.bindingAcceptedImpliesRuntimeAccepted
      artifact
      publicInput
      proof
      artifactAccepted
  exact
    runtime_artifact_checked_acceptance_evidence
      validation.transcriptValidation.artifactBindingValidation.runtimeValidation
      artifact
      publicInput
      proof
      runtimeAccepted

theorem runtime_soundness_checked_acceptance_verifier_accepts
    {system : VerifierModel}
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        system.accepts publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have artifactAccepted :=
    validation.transcriptValidation.transcriptAcceptedImpliesArtifactBindingAccepted
      artifact
      publicInput
      proof
      checked.left
  have runtimeAccepted :=
    validation.transcriptValidation.artifactBindingValidation.bindingAcceptedImpliesRuntimeAccepted
      artifact
      publicInput
      proof
      artifactAccepted
  exact
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      validation.transcriptValidation.artifactBindingValidation.runtimeValidation
      artifact
      publicInput
      proof
      runtimeAccepted

theorem runtime_soundness_checked_acceptance_transcript_bound
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
        system.transcriptBound publicInput proof := by
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
  exact
    runtime_soundness_evidence_implies_transcript_bound
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      evidence

theorem runtime_soundness_checked_acceptance_public_input_bound
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
        system.publicInputBound publicInput proof := by
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
  exact
    runtime_soundness_evidence_implies_public_input_bound
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      evidence

theorem runtime_soundness_checked_acceptance_pcs_and_fri
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
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
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
  exact
    runtime_soundness_evidence_implies_pcs_and_fri
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      evidence

theorem runtime_soundness_checked_acceptance_external_source_requirement
    {system : VerifierModel}
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        ExternalSourceOpeningRequirement
          system
          validation.sourceValidation
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource checked
  exact checked.right

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

theorem runtime_soundness_checked_acceptance_runtime_artifact_core_contract
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
        RuntimeArtifactEvidence
          system
          validation.transcriptValidation.artifactBindingValidation.runtimeValidation
          artifact
          publicInput
          proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
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
  exact
    runtime_soundness_evidence_implies_runtime_artifact_core_contract
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      evidence

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

theorem runtime_soundness_checked_acceptance_audited_core_contract
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
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have audited :=
    runtime_soundness_checked_acceptance_audited_assumptions
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have coreContract :=
    sound_witness_implies_verifier_core_contract audited.right.right
  exact
    And.intro audited.left
      (And.intro audited.right.left
        (And.intro coreContract audited.right.right))

theorem runtime_soundness_checked_acceptance_verifier_sound_witness
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
        SoundWitness system publicInput proof := by
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
  exact sound.right

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

theorem runtime_soundness_checked_acceptance_accepts_core_sound_witness
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
        system.accepts publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have verifierAccepts :=
    runtime_soundness_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have coreContract :=
    runtime_soundness_checked_acceptance_core_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have soundWitness :=
    runtime_soundness_checked_acceptance_verifier_sound_witness
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  exact And.intro verifierAccepts (And.intro coreContract soundWitness)

theorem runtime_soundness_checked_acceptance_execution_obligations
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
        exists witness trace constraints,
          system.traceConsistent publicInput proof trace
            /\ system.constraintsSatisfied constraints trace
            /\ system.witnessMatchesTrace witness trace := by
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
  cases sound.right with
  | intro witness tail =>
    cases tail with
    | intro trace tail =>
      cases tail with
      | intro constraints evidence =>
        exact
          Exists.intro witness
            (Exists.intro trace
              (Exists.intro constraints
                (And.intro evidence.right.right.right.right.left
                  (And.intro
                    evidence.right.right.right.right.right.left
                    evidence.right.right.right.right.right.right))))

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

theorem runtime_soundness_required_external_source_pcs_sound
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
          ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have requiredSound :=
    runtime_soundness_required_external_source_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  have externalEvidence := requiredSound.right.left
  have pcsOpenings :=
    external_source_opening_evidence_implies_pcs_openings
      validation.sourceValidation
      publicInput
      proof
      externalEvidence
  exact
    And.intro externalEvidence
      (And.intro pcsOpenings requiredSound.right.right)

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

theorem runtime_soundness_required_external_source_accepts_core_sound_witness
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
          system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have verifierAccepts :=
    runtime_soundness_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have requiredSound :=
    runtime_soundness_required_external_source_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  have coreContract :=
    sound_witness_implies_verifier_core_contract requiredSound.right.right
  exact
    And.intro verifierAccepts
      (And.intro requiredSound.right.left
        (And.intro coreContract requiredSound.right.right))

end Lzvm

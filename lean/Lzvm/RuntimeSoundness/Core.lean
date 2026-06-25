/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AssumptionAudit
import Lzvm.ExternalSource
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
  rcases evidence with
    ⟨_transcriptEvidence,
      _artifactEvidence,
      _transcriptBound,
      _publicInputBound,
      _sourceRequirement,
      pcsOpenings,
      friQueries⟩
  exact ⟨pcsOpenings, friQueries⟩

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
  rcases evidence with
    ⟨_transcriptEvidence, artifactEvidence, _transcriptBound,
      _publicInputBound, _sourceRequirement, _pcsOpenings, _friQueries⟩
  exact artifactEvidence

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
  rcases evidence with
    ⟨_transcriptEvidence, _artifactEvidence, transcriptBound,
      _publicInputBound, _sourceRequirement, _pcsOpenings, _friQueries⟩
  exact transcriptBound

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
  rcases evidence with
    ⟨_transcriptEvidence, _artifactEvidence, _transcriptBound,
      publicInputBound, _sourceRequirement, _pcsOpenings, _friQueries⟩
  exact publicInputBound

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
  rcases evidence with
    ⟨_transcriptEvidence, _artifactEvidence, _transcriptBound,
      _publicInputBound, sourceRequirement, _pcsOpenings, _friQueries⟩
  exact sourceRequirement

theorem runtime_soundness_evidence_implies_binding_pcs_fri_contract
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
        system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof requiresExternalSource evidence
  rcases evidence with
    ⟨_transcriptEvidence,
      _artifactEvidence,
      transcriptBound,
      publicInputBound,
      _sourceRequirement,
      pcsOpenings,
      friQueries⟩
  exact ⟨transcriptBound, publicInputBound, pcsOpenings, friQueries⟩

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
  rcases evidence with
    ⟨_transcriptEvidence,
      _artifactEvidence,
      transcriptBound,
      publicInputBound,
      _sourceRequirement,
      pcsOpenings,
      friQueries⟩
  exact ⟨transcriptBound, publicInputBound, pcsOpenings, friQueries⟩

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
  exact
    runtime_transcript_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation.transcriptValidation
      artifact
      publicInput
      proof
      checked.left

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
  have auditedCrypto :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have transcriptAccepted := checked.left
  have sourceRequirement := checked.right
  have transcriptFull :=
    runtime_transcript_binding_checked_acceptance_full_contract
      assumptions
      validation.transcriptValidation
      artifact
      publicInput
      proof
      transcriptAccepted
  have transcriptBound :=
    runtime_transcript_binding_evidence_implies_transcript_bound
      validation.transcriptValidation
      artifact
      publicInput
      proof
      transcriptFull.left
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
  have pcsOpeningSound :=
    required_crypto_assumptions_pcs_opening_soundness auditedCrypto
  have friQuerySound :=
    required_crypto_assumptions_fri_query_soundness auditedCrypto
  have pcsOpenings :=
    pcsOpeningSound publicInput proof verifierAccepts
  have friQueries :=
    friQuerySound publicInput proof verifierAccepts
  have coreContract :=
    runtime_soundness_checked_acceptance_core_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have publicInputBound :=
    coreContract.right.left
  exact
    And.intro transcriptFull.left
      (And.intro transcriptFull.right.right.left
        (And.intro transcriptBound
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
  have transcriptFull :=
    runtime_transcript_binding_checked_acceptance_full_contract
      assumptions
      validation.transcriptValidation
      artifact
      publicInput
      proof
      checked.left
  exact And.intro evidence transcriptFull.right.right.right

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

theorem runtime_soundness_checked_acceptance_audited_soundness_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeSoundnessValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeSoundnessCheckedAcceptance
          system validation artifact publicInput proof requiresExternalSource ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeSoundnessEvidence
            system validation artifact publicInput proof requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have audited :=
    runtime_soundness_checked_acceptance_audited_assumptions
      assumptions validation artifact publicInput proof requiresExternalSource checked
  exact
    And.intro audited.left
      (And.intro
        (assumption_bundle_carries_required_semantic_evidence assumptions)
        audited.right)

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
    runtime_soundness_checked_acceptance_core_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
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

theorem runtime_soundness_checked_acceptance_proof_system_sound
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
        ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have proofSystemSound := abstract_verifier_sound assumptions
  have verifierAccepts :=
    runtime_soundness_checked_acceptance_verifier_accepts
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
  exact And.intro proofSystemSound (And.intro verifierAccepts soundWitness)

theorem runtime_soundness_checked_acceptance_audited_accepts_sound_witness_contract
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
          /\ ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have proofSystemSound :=
    runtime_soundness_checked_acceptance_proof_system_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  exact And.intro auditedAssumptions proofSystemSound

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

theorem runtime_soundness_checked_acceptance_full_soundness_contract
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
          /\ RuntimeArtifactSoundnessObligations
            system
            validation.transcriptValidation.artifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
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
  have artifactEvidence :=
    runtime_soundness_checked_acceptance_runtime_artifact_evidence
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
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
  have artifactObligations :
      RuntimeArtifactSoundnessObligations
        system
        validation.transcriptValidation.artifactBindingValidation.runtimeValidation
        artifact
        publicInput
        proof :=
    And.intro artifactEvidence (And.intro verifierAccepts coreContract)
  have executionObligations :=
    runtime_soundness_checked_acceptance_execution_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  exact
    And.intro sound.left
      (And.intro artifactObligations
        (And.intro coreContract
          (And.intro executionObligations sound.right)))

theorem runtime_soundness_checked_acceptance_accepts_full_soundness_contract
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
          /\ RuntimeSoundnessEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeArtifactSoundnessObligations
            system
            validation.transcriptValidation.artifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
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
  have fullContract :=
    runtime_soundness_checked_acceptance_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  exact And.intro verifierAccepts fullContract

theorem runtime_soundness_checked_acceptance_proof_system_full_soundness_contract
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
        ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ RuntimeSoundnessEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeArtifactSoundnessObligations
            system
            validation.transcriptValidation.artifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have proofSystemSound := abstract_verifier_sound assumptions
  have acceptsFullContract :=
    runtime_soundness_checked_acceptance_accepts_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  exact And.intro proofSystemSound acceptsFullContract

theorem runtime_soundness_checked_acceptance_audited_proof_system_contract
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
          /\ ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ RuntimeSoundnessEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeArtifactSoundnessObligations
            system
            validation.transcriptValidation.artifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have fullContract :=
    runtime_soundness_checked_acceptance_proof_system_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  exact And.intro auditedAssumptions fullContract

theorem runtime_soundness_checked_acceptance_audited_soundness_proof_system_contract
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
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ RuntimeSoundnessEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeArtifactSoundnessObligations
            system
            validation.transcriptValidation.artifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
  have auditedContract :=
    runtime_soundness_checked_acceptance_audited_proof_system_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  exact
    And.intro auditedContract.left
      (And.intro
        (assumption_bundle_carries_required_semantic_evidence assumptions)
        auditedContract.right)


end Lzvm

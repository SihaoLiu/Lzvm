/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.RuntimeSoundness.Core

/-!
Runtime soundness theorems for required external-source openings.
-/

namespace Lzvm

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
  exact
    runtime_soundness_checked_acceptance_core_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked

theorem runtime_soundness_required_external_source_evidence_core_and_sound
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
            /\ RuntimeVerifierCoreContract system publicInput proof
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
  have coreContract :=
    runtime_soundness_required_external_source_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact
    And.intro requiredSound.left
      (And.intro requiredSound.right.left
        (And.intro coreContract requiredSound.right.right))

theorem runtime_soundness_required_external_source_evidence_audited_core_contract
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
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
            /\ RuntimeSoundnessEvidence
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
            /\ RuntimeVerifierCoreContract system publicInput proof
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
  have coreContract :=
    runtime_soundness_required_external_source_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (And.intro
        (assumption_bundle_carries_required_semantic_evidence assumptions)
        (And.intro requiredSound.left
          (And.intro requiredSound.right.left
            (And.intro coreContract requiredSound.right.right))))

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
    runtime_soundness_required_external_source_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact
    And.intro verifierAccepts
      (And.intro requiredSound.right.left
        (And.intro coreContract requiredSound.right.right))

theorem runtime_soundness_required_external_source_full_soundness_contract
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
  have fullContract :=
    runtime_soundness_checked_acceptance_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  exact
    And.intro verifierAccepts
      (And.intro requiredSound.right.left fullContract)

theorem runtime_soundness_required_external_source_proof_system_full_soundness_contract
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
          ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
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
  intro artifact publicInput proof requiresExternalSource checked required
  have proofSystemSound := abstract_verifier_sound assumptions
  have fullContract :=
    runtime_soundness_required_external_source_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact And.intro proofSystemSound fullContract

theorem runtime_soundness_required_external_source_audited_proof_system_contract
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
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
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
  intro artifact publicInput proof requiresExternalSource checked required
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have fullContract :=
    runtime_soundness_required_external_source_proof_system_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact And.intro auditedAssumptions fullContract

theorem runtime_soundness_required_external_source_audited_soundness_proof_system_contract
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
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
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
  intro artifact publicInput proof requiresExternalSource checked required
  have auditedContract :=
    runtime_soundness_required_external_source_audited_proof_system_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (And.intro
        (assumption_bundle_carries_required_semantic_evidence assumptions)
        auditedContract.right)

theorem runtime_soundness_required_external_source_audited_accepts_sound_witness_contract
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
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have proofSystemSound := abstract_verifier_sound assumptions
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
  exact
    And.intro auditedAssumptions
      (And.intro proofSystemSound
        (And.intro verifierAccepts
          (And.intro requiredSound.right.left requiredSound.right.right)))

theorem runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract
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
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have proofSystemSound := abstract_verifier_sound assumptions
  have verifierAccepts :=
    runtime_soundness_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have pcsSound :=
    runtime_soundness_required_external_source_pcs_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact
    And.intro auditedAssumptions
      (And.intro proofSystemSound
          (And.intro verifierAccepts
            (And.intro pcsSound.left
              (And.intro pcsSound.right.left pcsSound.right.right))))

theorem runtime_soundness_required_external_source_audited_pcs_fri_witness_contract
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
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have proofSystemSound := abstract_verifier_sound assumptions
  have verifierAccepts :=
    runtime_soundness_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have pcsSound :=
    runtime_soundness_required_external_source_pcs_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  have pcsAndFri :=
    runtime_soundness_checked_acceptance_pcs_and_fri
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  exact
    And.intro auditedAssumptions
      (And.intro proofSystemSound
        (And.intro verifierAccepts
          (And.intro pcsSound.left
            (And.intro pcsSound.right.left
              (And.intro pcsAndFri.right pcsSound.right.right)))))

theorem runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract
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
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have compactContract :=
    runtime_soundness_required_external_source_audited_pcs_fri_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  have coreContract :=
    runtime_soundness_required_external_source_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact
    And.intro compactContract.left
      (And.intro compactContract.right.left
        (And.intro compactContract.right.right.left
          (And.intro compactContract.right.right.right.left
            (And.intro compactContract.right.right.right.right.left
              (And.intro compactContract.right.right.right.right.right.left
                (And.intro coreContract
                  compactContract.right.right.right.right.right.right))))))

theorem runtime_soundness_required_external_source_audited_soundness_pcs_fri_core_witness_contract
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
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have compactContract :=
    runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (And.intro
        (assumption_bundle_carries_required_semantic_evidence assumptions)
        compactContract.right)

theorem runtime_soundness_required_external_source_audited_proof_system_core_contract
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
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ system.transcriptBound publicInput proof
            /\ system.publicInputBound publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ (exists witness trace constraints,
              system.traceConsistent publicInput proof trace
                /\ system.constraintsSatisfied constraints trace
                /\ system.witnessMatchesTrace witness trace)
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have compactContract :=
    runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  have transcriptBound :=
    runtime_soundness_checked_acceptance_transcript_bound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have publicInputBound :=
    runtime_soundness_checked_acceptance_public_input_bound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
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
    And.intro compactContract.left
      (And.intro compactContract.right.left
        (And.intro compactContract.right.right.left
          (And.intro compactContract.right.right.right.left
            (And.intro transcriptBound
              (And.intro publicInputBound
                (And.intro compactContract.right.right.right.right.left
                  (And.intro compactContract.right.right.right.right.right.left
                    (And.intro compactContract.right.right.right.right.right.right.left
                      (And.intro executionObligations
                        compactContract.right.right.right.right.right.right.right)))))))))

theorem runtime_soundness_required_external_source_audited_finalized_core_sound_witness_contract
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
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
            /\ RuntimeProofArtifactFinalized
              system
              validation.transcriptValidation.artifactBindingValidation
              artifact
              publicInput
              proof
            /\ ExternalSourceOpeningEvidence
              system
              validation.sourceValidation
              publicInput
              proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ (exists witness trace constraints,
              system.traceConsistent publicInput proof trace
                /\ system.constraintsSatisfied constraints trace
                /\ system.witnessMatchesTrace witness trace)
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have finalizedContract :=
    runtime_soundness_checked_acceptance_audited_finalized_core_sound_witness_contract
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
  rcases finalizedContract with
    ⟨cryptoEvidence,
      semanticEvidence,
      artifactFinalized,
      coreContract,
      executionObligations,
      soundWitness⟩
  exact
    And.intro cryptoEvidence
      (And.intro semanticEvidence
        (And.intro artifactFinalized
          (And.intro externalEvidence
            (And.intro coreContract
              (And.intro executionObligations soundWitness)))))


end Lzvm

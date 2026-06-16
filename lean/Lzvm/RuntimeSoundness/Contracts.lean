/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.RuntimeSoundness

/-!
Compact runtime soundness contracts.
-/

namespace Lzvm

theorem runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract
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
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked
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
  have pcsAndFri :=
    runtime_soundness_checked_acceptance_pcs_and_fri
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
  have coreContract :=
    runtime_soundness_checked_acceptance_verifier_core_contract
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
  exact
    And.intro auditedAssumptions
      (And.intro proofSystemSound
        (And.intro verifierAccepts
          (And.intro transcriptBound
            (And.intro publicInputBound
              (And.intro pcsAndFri.left
                (And.intro pcsAndFri.right
                  (And.intro coreContract soundWitness)))))))

theorem runtime_soundness_checked_acceptance_contracts_core_contract
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
  intro artifact publicInput proof requiresExternalSource checked
  have compactContract :=
    runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract
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
  rcases compactContract with
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      verifierCore,
      soundWitness⟩
  exact
    And.intro auditedAssumptions
      (And.intro proofSystemSound
        (And.intro verifierAccepts
          (And.intro transcriptBound
            (And.intro publicInputBound
              (And.intro pcsOpenings
                (And.intro friQueries
                  (And.intro verifierCore
                    (And.intro executionObligations soundWitness))))))))

theorem runtime_soundness_required_external_source_contracts_core_contract
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
  exact
    runtime_soundness_required_external_source_audited_proof_system_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required

end Lzvm

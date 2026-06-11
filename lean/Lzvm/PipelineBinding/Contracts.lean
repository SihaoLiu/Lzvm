/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.PipelineBinding

/-!
Compact proof-system contracts derived from runtime proof pipeline binding.
-/

namespace Lzvm

theorem runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof (requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ RuntimePipelineBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeArtifactSoundnessObligations
            system
            validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have proofSystemSound := abstract_verifier_sound assumptions
  have acceptsFullContract :=
    runtime_pipeline_binding_checked_acceptance_accepts_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact And.intro proofSystemSound acceptsFullContract

theorem runtime_pipeline_binding_checked_acceptance_audited_proof_system_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ RuntimePipelineBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeArtifactSoundnessObligations
            system
            validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have fullContract :=
    runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact And.intro auditedAssumptions fullContract

theorem runtime_pipeline_binding_checked_acceptance_audited_proof_system_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
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
  intro artifact publicInput proof _requiresExternalSource accepted
  have compactContract :=
    runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      _requiresExternalSource
      accepted
  have executionObligations :=
    runtime_pipeline_binding_checked_acceptance_execution_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro compactContract.left
      (And.intro compactContract.right.left
        (And.intro compactContract.right.right.left
          (And.intro compactContract.right.right.right.left
            (And.intro compactContract.right.right.right.right.left
              (And.intro compactContract.right.right.right.right.right.left
                (And.intro compactContract.right.right.right.right.right.right.left
                  (And.intro compactContract.right.right.right.right.right.right.right.left
                    (And.intro executionObligations
                      compactContract.right.right.right.right.right.right.right.right))))))))

end Lzvm

/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.PipelineBinding

/-!
Checked-acceptance contracts derived from runtime proof pipeline binding.
-/

namespace Lzvm

theorem runtime_pipeline_binding_checked_acceptance_accepts_full_soundness_contract
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
        system.accepts publicInput proof
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
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have fullContract :=
    runtime_pipeline_binding_checked_acceptance_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  rcases fullContract with
    ⟨pipelineEvidence,
      artifactObligations,
      coreContract,
      executionObligations,
      soundWitness,
      _foldTraceIdentityContract⟩
  exact
    And.intro verifierAccepts
      (And.intro pipelineEvidence
        (And.intro artifactObligations
          (And.intro coreContract
            (And.intro executionObligations soundWitness))))

theorem runtime_pipeline_binding_checked_acceptance_proof_system_sound
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
        ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof _requiresExternalSource accepted
  have proofSystemSound := abstract_verifier_sound assumptions
  have acceptedSound :=
    runtime_pipeline_binding_checked_acceptance_verifier_sound_witness
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact And.intro proofSystemSound acceptedSound

theorem runtime_pipeline_binding_checked_acceptance_audited_accepts_sound_witness_contract
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
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof _requiresExternalSource accepted
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have proofSystemSound :=
    runtime_pipeline_binding_checked_acceptance_proof_system_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      _requiresExternalSource
      accepted
  exact And.intro auditedAssumptions proofSystemSound

theorem runtime_pipeline_binding_checked_acceptance_audited_soundness_accepts_contract
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
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ ProofSystemSound system
          /\ system.accepts publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof _requiresExternalSource accepted
  have audited :=
    runtime_pipeline_binding_checked_acceptance_audited_accepts_sound_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      _requiresExternalSource
      accepted
  have auditedAssumptions :=
    assumption_bundle_carries_required_evidence assumptions
  exact
    And.intro auditedAssumptions.left
      (And.intro auditedAssumptions.right audited.right)

end Lzvm

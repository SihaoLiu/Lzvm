/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.PipelineBinding.Accepts

/-!
Compact proof-system contracts derived from runtime proof pipeline binding.
-/

namespace Lzvm

theorem runtime_pipeline_binding_checked_acceptance_opening_bound_contract
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningBoundContract
          system
          validation.queryPlanBindingValidation.openingValidation.openingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have openingSegmentAccepted :=
    runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_segment_binding_checked_acceptance_opening_bound_contract
      validation.queryPlanBindingValidation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentAccepted

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

theorem runtime_pipeline_binding_evidence_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RequiredCryptographicAssumptionStatements assumptions.crypto
        /\ system.transcriptBound publicInput proof
        /\ system.publicInputBound publicInput proof
        /\ system.pcsOpeningsValid publicInput proof
        /\ system.friQueriesValid publicInput proof
        /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro evidence
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have transcriptBound :=
    runtime_pipeline_binding_evidence_implies_transcript_bound evidence
  have publicInputBound :=
    runtime_pipeline_binding_evidence_implies_public_input_bound evidence
  have pcsAndFri :=
    runtime_pipeline_binding_evidence_implies_pcs_and_fri evidence
  have coreContract :=
    runtime_pipeline_binding_evidence_implies_core_obligations evidence
  exact
    And.intro auditedAssumptions
      (And.intro transcriptBound
        (And.intro publicInputBound
          (And.intro pcsAndFri.left
            (And.intro pcsAndFri.right coreContract))))

theorem runtime_pipeline_binding_checked_acceptance_audited_binding_pcs_fri_core_witness_contract
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
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof _requiresExternalSource accepted
  have compactContract :=
    runtime_pipeline_binding_checked_acceptance_audited_accepts_sound_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      _requiresExternalSource
      accepted
  have transcriptBound :=
    runtime_pipeline_binding_checked_acceptance_transcript_bound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have publicInputBound :=
    runtime_pipeline_binding_checked_acceptance_public_input_bound
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have pcsAndFri :=
    runtime_pipeline_binding_checked_acceptance_pcs_and_fri
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have coreContract :=
    runtime_pipeline_binding_checked_acceptance_verifier_core_contract
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
          (And.intro transcriptBound
            (And.intro publicInputBound
              (And.intro pcsAndFri.left
                (And.intro pcsAndFri.right
                  (And.intro coreContract.right
                    compactContract.right.right.right)))))))

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

theorem runtime_pipeline_binding_required_external_source_contracts_core_contract
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
        requiresExternalSource ->
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ ProofSystemSound system
            /\ system.accepts publicInput proof
            /\ ExternalSourceOpeningEvidence
              system
              (runtime_pipeline_trace_source_validation validation)
              publicInput
              proof
            /\ ExternalSourceOpeningEvidence
              system
              (runtime_pipeline_opening_source_validation validation)
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
  intro artifact publicInput proof requiresExternalSource accepted required
  exact
    runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required

end Lzvm

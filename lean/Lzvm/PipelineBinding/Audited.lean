/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.PipelineBinding.Obligations

/-!
Audited runtime proof pipeline binding contracts.
-/

namespace Lzvm

theorem runtime_pipeline_binding_required_external_source_full_soundness_contract
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
          system.accepts publicInput proof
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
  intro artifact publicInput proof requiresExternalSource accepted required
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have requiredSound :=
    runtime_pipeline_binding_required_external_source_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have fullContract :=
    runtime_pipeline_binding_checked_acceptance_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  cases requiredSound with
  | intro _pipelineEvidence tail =>
    cases tail with
    | intro traceExternalEvidence tail =>
      cases tail with
      | intro openingExternalEvidence _tail =>
        exact
          And.intro verifierAccepts
            (And.intro traceExternalEvidence
              (And.intro openingExternalEvidence fullContract))

theorem runtime_pipeline_binding_required_external_source_proof_system_full_soundness_contract
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
          ProofSystemSound system
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
  intro artifact publicInput proof requiresExternalSource accepted required
  have proofSystemSound := abstract_verifier_sound assumptions
  have fullContract :=
    runtime_pipeline_binding_required_external_source_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  exact And.intro proofSystemSound fullContract

theorem runtime_pipeline_binding_required_external_source_audited_proof_system_contract
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
  intro artifact publicInput proof requiresExternalSource accepted required
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have fullContract :=
    runtime_pipeline_binding_required_external_source_proof_system_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  exact And.intro auditedAssumptions fullContract

theorem runtime_pipeline_binding_required_external_source_audited_accepts_sound_witness_contract
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
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have proofSystemSound := abstract_verifier_sound assumptions
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have requiredSound :=
    runtime_pipeline_binding_required_external_source_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  exact
    And.intro auditedAssumptions
      (And.intro proofSystemSound
        (And.intro verifierAccepts
          (And.intro requiredSound.right.left
            (And.intro requiredSound.right.right.left
              requiredSound.right.right.right.right))))

theorem runtime_pipeline_binding_required_external_source_audited_pcs_accepts_sound_witness_contract
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
            /\ system.pcsOpeningsValid publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have proofSystemSound := abstract_verifier_sound assumptions
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have requiredSound :=
    runtime_pipeline_binding_required_external_source_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  exact
    And.intro auditedAssumptions
      (And.intro proofSystemSound
        (And.intro verifierAccepts
          (And.intro requiredSound.right.left
            (And.intro requiredSound.right.right.left
              (And.intro requiredSound.right.right.right.left
                requiredSound.right.right.right.right)))))

theorem runtime_pipeline_binding_required_external_source_audited_pcs_fri_witness_contract
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
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have auditedAssumptions :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have proofSystemSound := abstract_verifier_sound assumptions
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have requiredSound :=
    runtime_pipeline_binding_required_external_source_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have pcsAndFri :=
    runtime_pipeline_binding_checked_acceptance_pcs_and_fri
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro auditedAssumptions
      (And.intro proofSystemSound
        (And.intro verifierAccepts
          (And.intro requiredSound.right.left
            (And.intro requiredSound.right.right.left
              (And.intro requiredSound.right.right.right.left
                (And.intro pcsAndFri.right
                  requiredSound.right.right.right.right))))))

theorem runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract
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
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked
              artifact
              publicInput
              proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have compactContract :=
    runtime_pipeline_binding_required_external_source_audited_pcs_fri_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have coreContract :=
    runtime_pipeline_binding_required_external_source_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have seedBinds :=
    runtime_pipeline_binding_checked_acceptance_seed_binds_witness_tree_digests
      validation
      artifact
      publicInput
      proof
      accepted
  have seededFriOpeningChecked :=
    runtime_pipeline_binding_checked_acceptance_seeded_fri_opening_requirements_checked
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
                  (And.intro seedBinds
                      (And.intro seededFriOpeningChecked
                        (And.intro coreContract.right.right
                          compactContract.right.right.right.right.right.right.right)))))))))

theorem runtime_pipeline_binding_required_external_source_audited_seeded_query_requirements_contract
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
          validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked
              artifact
              publicInput
              proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have auditedCore :=
    runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have seedBinds :=
    auditedCore.right.right.right.right.right.right.right.left
  have seededFriOpeningChecked :=
    auditedCore.right.right.right.right.right.right.right.right.left
  exact And.intro seedBinds seededFriOpeningChecked

theorem runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract
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
            /\ validation.queryPlanBindingValidation.queryPlanSeedBindsWitnessTreeDigests
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanSeededFriOpeningRequirementsChecked
              artifact
              publicInput
              proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ (exists witness trace constraints,
              system.traceConsistent publicInput proof trace
                /\ system.constraintsSatisfied constraints trace
                /\ system.witnessMatchesTrace witness trace)
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have compactContract :=
    runtime_pipeline_binding_required_external_source_audited_pcs_fri_core_witness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
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
  have executionObligations :=
    runtime_pipeline_binding_checked_acceptance_execution_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  have seedBinds :=
    compactContract.right.right.right.right.right.right.right.left
  have seededFriOpeningChecked :=
    compactContract.right.right.right.right.right.right.right.right.left
  have verifierCore :=
    compactContract.right.right.right.right.right.right.right.right.right.left
  have soundWitness :=
    compactContract.right.right.right.right.right.right.right.right.right.right
  exact
    And.intro compactContract.left
      (And.intro compactContract.right.left
        (And.intro compactContract.right.right.left
          (And.intro compactContract.right.right.right.left
            (And.intro compactContract.right.right.right.right.left
              (And.intro transcriptBound
                  (And.intro publicInputBound
                    (And.intro compactContract.right.right.right.right.right.left
                      (And.intro compactContract.right.right.right.right.right.right.left
                        (And.intro seedBinds
                          (And.intro seededFriOpeningChecked
                            (And.intro verifierCore
                              (And.intro executionObligations soundWitness))))))))))))


end Lzvm

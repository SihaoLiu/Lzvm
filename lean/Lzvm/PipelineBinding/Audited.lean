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
              /\ SoundWitness system publicInput proof
              /\ RuntimeFriFoldTraceIdentityContract
                system
                validation.queryPlanBindingValidation.openingValidation
                artifact
                publicInput
                proof
              /\ RuntimeFriFoldQueryPlanOrderContract
                system
                validation.queryPlanBindingValidation.openingValidation
                artifact
                publicInput
                proof := by
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
  rcases requiredSound with
    ⟨_pipelineEvidence,
      traceExternalEvidence,
      openingExternalEvidence,
      _pcsOpenings,
      _soundWitness⟩
  rcases fullContract with
    ⟨pipelineEvidence,
      artifactObligations,
      coreContract,
      executionObligations,
      soundWitness,
      foldTraceIdentityContract,
      foldQueryPlanOrderContract⟩
  exact
    ⟨verifierAccepts,
      traceExternalEvidence,
      openingExternalEvidence,
      pipelineEvidence,
      artifactObligations,
      coreContract,
      executionObligations,
      soundWitness,
      foldTraceIdentityContract,
      foldQueryPlanOrderContract⟩

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
              /\ SoundWitness system publicInput proof
              /\ RuntimeFriFoldTraceIdentityContract
                system
                validation.queryPlanBindingValidation.openingValidation
                artifact
                publicInput
                proof
              /\ RuntimeFriFoldQueryPlanOrderContract
                system
                validation.queryPlanBindingValidation.openingValidation
                artifact
                publicInput
                proof := by
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
              /\ SoundWitness system publicInput proof
              /\ RuntimeFriFoldTraceIdentityContract
                system
                validation.queryPlanBindingValidation.openingValidation
                artifact
                publicInput
                proof
              /\ RuntimeFriFoldQueryPlanOrderContract
                system
                validation.queryPlanBindingValidation.openingValidation
                artifact
                publicInput
                proof := by
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

theorem runtime_pipeline_binding_required_external_source_audited_soundness_proof_system_contract
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
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
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
              /\ SoundWitness system publicInput proof
              /\ RuntimeFriFoldTraceIdentityContract
                system
                validation.queryPlanBindingValidation.openingValidation
                artifact
                publicInput
                proof
              /\ RuntimeFriFoldQueryPlanOrderContract
                system
                validation.queryPlanBindingValidation.openingValidation
                artifact
                publicInput
                proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have auditedContract :=
    runtime_pipeline_binding_required_external_source_audited_proof_system_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have auditedAssumptions :=
    assumption_bundle_carries_required_evidence assumptions
  exact
    And.intro auditedAssumptions.left
      (And.intro auditedAssumptions.right auditedContract.right)

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
  rcases requiredSound with
    ⟨_pipelineEvidence,
      traceExternalEvidence,
      openingExternalEvidence,
      _pcsOpenings,
      soundWitness⟩
  exact
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      traceExternalEvidence,
      openingExternalEvidence,
      soundWitness⟩

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
  rcases requiredSound with
    ⟨_pipelineEvidence,
      traceExternalEvidence,
      openingExternalEvidence,
      pcsOpenings,
      soundWitness⟩
  exact
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      traceExternalEvidence,
      openingExternalEvidence,
      pcsOpenings,
      soundWitness⟩

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
  rcases requiredSound with
    ⟨_pipelineEvidence,
      traceExternalEvidence,
      openingExternalEvidence,
      pcsOpenings,
      soundWitness⟩
  rcases pcsAndFri with ⟨_pcsOpeningsFromChecked, friQueries⟩
  exact
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      traceExternalEvidence,
      openingExternalEvidence,
      pcsOpenings,
      friQueries,
      soundWitness⟩

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
  rcases compactContract with
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      traceExternalEvidence,
      openingExternalEvidence,
      pcsOpenings,
      friQueries,
      soundWitness⟩
  rcases coreContract with
    ⟨_traceExternalCore, _openingExternalCore, verifierCore⟩
  exact
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      traceExternalEvidence,
      openingExternalEvidence,
      pcsOpenings,
      friQueries,
      seedBinds,
      seededFriOpeningChecked,
      verifierCore,
      soundWitness⟩

theorem runtime_pipeline_required_external_source_audited_finalized_core_sound_witness_contract
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
          let queryPlanValidation := validation.queryPlanBindingValidation
          let artifactValidation :=
            queryPlanValidation.challengeValidation.transcriptValidation.artifactBindingValidation
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
            /\ RuntimeProofArtifactFinalized
              system
              artifactValidation
              artifact
              publicInput
              proof
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
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have auditedCoreSound :=
    accepted_proof_audited_core_and_sound_witness
      assumptions
      publicInput
      proof
      verifierAccepts
  have requiredCore :=
    runtime_pipeline_binding_required_external_source_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have artifactFinalized :=
    runtime_pipeline_binding_checked_acceptance_artifact_finalized
      validation
      artifact
      publicInput
      proof
      accepted
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
  rcases auditedCoreSound with
    ⟨cryptoEvidence, semanticEvidence, verifierCore, soundWitness⟩
  rcases requiredCore with
    ⟨traceExternalEvidence, openingExternalEvidence, _requiredVerifierCore⟩
  have executionObligations :=
    sound_witness_implies_execution_obligations soundWitness
  exact
    ⟨cryptoEvidence,
      semanticEvidence,
      artifactFinalized,
      traceExternalEvidence,
      openingExternalEvidence,
      seedBinds,
      seededFriOpeningChecked,
      verifierCore,
      executionObligations,
      soundWitness⟩

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
  rcases auditedCore with
    ⟨_auditedAssumptions,
      _proofSystemSound,
      _verifierAccepts,
      _traceExternalEvidence,
      _openingExternalEvidence,
      _pcsOpenings,
      _friQueries,
      seedBinds,
      seededFriOpeningChecked,
      _verifierCore,
      _soundWitness⟩
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
  rcases compactContract with
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      traceExternalEvidence,
      openingExternalEvidence,
      pcsOpenings,
      friQueries,
      seedBinds,
      seededFriOpeningChecked,
      verifierCore,
      soundWitness⟩
  exact
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      traceExternalEvidence,
      openingExternalEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      seedBinds,
      seededFriOpeningChecked,
      verifierCore,
      executionObligations,
      soundWitness⟩


end Lzvm

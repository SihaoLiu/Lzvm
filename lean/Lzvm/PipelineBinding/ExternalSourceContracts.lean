/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.PipelineBinding.Contracts

/-!
External-source pipeline contracts derived from runtime proof pipeline binding.
-/

namespace Lzvm

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
  have auditedCore :=
    runtime_pipeline_binding_required_external_source_audited_proof_system_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have externalCore :=
    runtime_pipeline_binding_required_external_source_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  rcases externalCore with
    ⟨traceExternalEvidence,
      openingExternalEvidence,
      verifierCore⟩
  rcases auditedCore with
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      _traceExternalEvidence,
      _openingExternalEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpenings,
      friQueries,
      seedBinds,
      seededFriOpeningChecked,
      _auditedVerifierCore,
      executionObligations,
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

theorem runtime_pipeline_binding_required_external_source_contracts_audited_soundness_core_contract
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
  have auditedSoundness :=
    runtime_pipeline_binding_required_external_source_audited_soundness_proof_system_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have compactCore :=
    runtime_pipeline_binding_required_external_source_contracts_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  rcases compactCore with
    ⟨auditedCrypto,
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
  exact
    ⟨auditedCrypto,
      auditedSoundness.right.left,
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

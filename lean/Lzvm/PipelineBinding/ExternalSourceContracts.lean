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

theorem runtime_pipeline_binding_required_external_source_contracts_manifest_core_contract
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
            /\ RuntimeQueryPlanMaterialManifestContract
              system
              validation.queryPlanBindingValidation
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanSegmentCanonical
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanMaterialManifestMatchesSchedule
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
  have coreContract :=
    runtime_pipeline_binding_required_external_source_contracts_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have materialManifestComponents :=
    runtime_pipeline_binding_checked_acceptance_query_plan_material_manifest_components
      validation
      artifact
      publicInput
      proof
      accepted
  rcases materialManifestComponents with
    ⟨materialManifest, segmentCanonical, materialManifestMatches⟩
  rcases coreContract with
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
  exact
    ⟨auditedAssumptions,
      proofSystemSound,
      verifierAccepts,
      materialManifest,
      segmentCanonical,
      materialManifestMatches,
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

theorem runtime_pipeline_required_external_source_manifest_audited_core_contract
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
            /\ RuntimeQueryPlanMaterialManifestContract
              system
              validation.queryPlanBindingValidation
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanSegmentCanonical
              artifact
              publicInput
              proof
            /\ validation.queryPlanBindingValidation.queryPlanMaterialManifestMatchesSchedule
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
  have manifestCore :=
    runtime_pipeline_binding_required_external_source_contracts_manifest_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  have auditedCrypto :=
    assumption_bundle_carries_required_crypto_evidence assumptions
  have auditedSemantic :=
    assumption_bundle_carries_required_semantic_evidence assumptions
  rcases manifestCore with
    ⟨_auditedCrypto,
      proofSystemSound,
      verifierAccepts,
      materialManifest,
      segmentCanonical,
      materialManifestMatches,
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
      auditedSemantic,
      proofSystemSound,
      verifierAccepts,
      materialManifest,
      segmentCanonical,
      materialManifestMatches,
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
    ⟨_auditedCrypto,
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
    ⟨assumption_bundle_carries_required_crypto_evidence assumptions,
      assumption_bundle_carries_required_semantic_evidence assumptions,
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

theorem runtime_pipeline_binding_required_external_source_artifact_contracts_core_contract
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
          RuntimeArtifactEvidence
            system
            validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
            /\ RequiredCryptographicAssumptionStatements assumptions.crypto
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
  have artifactEvidence :=
    runtime_pipeline_binding_checked_acceptance_runtime_artifact_evidence
      validation
      artifact
      publicInput
      proof
      accepted
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
  exact And.intro artifactEvidence compactCore

theorem runtime_pipeline_binding_required_external_source_artifact_audited_soundness_core_contract
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
          RuntimeArtifactEvidence
            system
            validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
            artifact
            publicInput
            proof
            /\ RequiredCryptographicAssumptionStatements assumptions.crypto
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
  have artifactCore :=
    runtime_pipeline_binding_required_external_source_artifact_contracts_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
      required
  rcases artifactCore with
    ⟨artifactEvidence,
      _auditedCrypto,
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
    ⟨artifactEvidence,
      assumption_bundle_carries_required_crypto_evidence assumptions,
      assumption_bundle_carries_required_semantic_evidence assumptions,
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
